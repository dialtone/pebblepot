//! The board read model: everything the family board shows.

use std::collections::HashMap;

use jiff::civil::Date;
use rusqlite::OptionalExtension;

use crate::model::ItemKind;
use crate::store::{Result, Store};

#[derive(Clone, Debug)]
pub struct Member {
    pub id: i64,
    pub name: String,
    pub face: String,
    pub colour: String,
    /// The sum of this family member's log entries.
    pub pot: i64,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub id: i64,
    pub name: String,
    pub picture: String,
    pub value: i64,
}

#[derive(Clone, Debug)]
pub struct Board {
    pub members: Vec<Member>,
    pub daily_chores: Vec<Item>,
    pub good_deeds: Vec<Item>,
    pub occasional_chores: Vec<Item>,
    /// (member id, item id) -> log entry id, for every chore earned on the given day.
    pub done: HashMap<(i64, i64), i64>,
    /// (member id, good deed id) -> that person's count for the given day, for every good deed
    /// earned at least once that day.
    pub good_deed_counts: HashMap<(i64, i64), GoodDeedCount>,
}

/// One family member's tally for one good deed on one day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GoodDeedCount {
    pub count: i64,
    /// The most recent of those entries: the one the badge takes away.
    pub latest_entry: i64,
}

/// The board as it stands on the household's local day `today`.
pub async fn load(store: &Store, today: Date) -> Result<Board> {
    let today = today.to_string();
    store
        .call(move |conn| {
            let members = conn
                .prepare(
                    "SELECT m.id, m.name, m.face, m.colour, COALESCE(SUM(l.amount), 0)
                     FROM family_member m LEFT JOIN log_entry l ON l.member_id = m.id
                     WHERE m.retired = 0
                     GROUP BY m.id ORDER BY m.position, m.id",
                )?
                .query_map([], |r| {
                    Ok(Member {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        face: r.get(2)?,
                        colour: r.get(3)?,
                        pot: r.get(4)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;

            let items = |kind: ItemKind| -> rusqlite::Result<Vec<Item>> {
                conn.prepare_cached(
                    "SELECT id, name, picture, value FROM item
                     WHERE kind = ?1 AND retired = 0 ORDER BY position, id",
                )?
                .query_map([kind.as_str()], |r| {
                    Ok(Item {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        picture: r.get(2)?,
                        value: r.get(3)?,
                    })
                })?
                .collect()
            };

            let done = conn
                .prepare(
                    "SELECT l.member_id, l.item_id, l.id FROM log_entry l JOIN item i ON i.id = l.item_id
                     WHERE l.entry_type = 'earn' AND l.local_day = ?1 AND i.kind IN (?2, ?3)",
                )?
                .query_map(
                    (
                        &today,
                        ItemKind::DailyChore.as_str(),
                        ItemKind::OccasionalChore.as_str(),
                    ),
                    |r| Ok(((r.get(0)?, r.get(1)?), r.get(2)?)),
                )?
                .collect::<rusqlite::Result<HashMap<_, _>>>()?;

            let good_deed_counts = conn
                .prepare(
                    "SELECT l.member_id, l.item_id, COUNT(*), MAX(l.id)
                     FROM log_entry l JOIN item i ON i.id = l.item_id
                     WHERE l.entry_type = 'earn' AND l.local_day = ?1 AND i.kind = ?2
                     GROUP BY l.member_id, l.item_id",
                )?
                .query_map((&today, ItemKind::GoodDeed.as_str()), |r| {
                    Ok((
                        (r.get(0)?, r.get(1)?),
                        GoodDeedCount {
                            count: r.get(2)?,
                            latest_entry: r.get(3)?,
                        },
                    ))
                })?
                .collect::<rusqlite::Result<HashMap<_, _>>>()?;

            Ok(Board {
                daily_chores: items(ItemKind::DailyChore)?,
                good_deeds: items(ItemKind::GoodDeed)?,
                occasional_chores: items(ItemKind::OccasionalChore)?,
                members,
                done,
                good_deed_counts,
            })
        })
        .await
}

/// One of today's log entries, as the today list shows it.
#[derive(Clone, Debug)]
pub struct TodayEntry {
    pub id: i64,
    pub name: String,
    pub picture: String,
    pub amount: i64,
}

/// A family member's today list: every log entry from `today`, most recent first.
#[derive(Clone, Debug)]
pub struct TodayList {
    pub member: Member,
    pub entries: Vec<TodayEntry>,
}

/// The today list for `member_id`, or `None` if they aren't on the board.
pub async fn today_list(store: &Store, member_id: i64, today: Date) -> Result<Option<TodayList>> {
    let today = today.to_string();
    store
        .call(move |conn| {
            let member = conn
                .query_row(
                    "SELECT m.id, m.name, m.face, m.colour,
                            (SELECT COALESCE(SUM(amount), 0) FROM log_entry WHERE member_id = m.id)
                     FROM family_member m WHERE m.id = ?1 AND m.retired = 0",
                    [member_id],
                    |r| {
                        Ok(Member {
                            id: r.get(0)?,
                            name: r.get(1)?,
                            face: r.get(2)?,
                            colour: r.get(3)?,
                            pot: r.get(4)?,
                        })
                    },
                )
                .optional()?;
            let Some(member) = member else {
                return Ok(None);
            };
            let entries = conn
                .prepare(
                    "SELECT id, name, picture, amount FROM log_entry
                     WHERE member_id = ?1 AND local_day = ?2 ORDER BY id DESC",
                )?
                .query_map((member_id, &today), |r| {
                    Ok(TodayEntry {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        picture: r.get(2)?,
                        amount: r.get(3)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(Some(TodayList { member, entries }))
        })
        .await
}

/// Every active reward, in Spend sheet order.
pub async fn rewards(store: &Store) -> Result<Vec<Item>> {
    store
        .call(|conn| {
            Ok(conn
                .prepare(
                    "SELECT id, name, picture, value FROM item
                     WHERE kind = ?1 AND retired = 0 ORDER BY position, id",
                )?
                .query_map([ItemKind::Reward.as_str()], |r| {
                    Ok(Item {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        picture: r.get(2)?,
                        value: r.get(3)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .await
}
