//! The pebble rules: every command that changes the log, each in one immediate transaction.

use jiff::Timestamp;
use jiff::tz::TimeZone;
use rusqlite::{OptionalExtension, TransactionBehavior};

use crate::clock::local_day;
use crate::model::ItemKind;
use crate::store::{self, Store};

/// Why a command was refused. Shown to the family as a short message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    AlreadyDoneToday,
    UnknownMember,
    UnknownItem,
    /// Already undone, or never there.
    UnknownEntry,
    /// The entry is from an earlier local day: the past can't be rewritten.
    NotUndoable,
}

impl Refusal {
    /// Stable code for templates and tests.
    pub fn code(self) -> &'static str {
        match self {
            Refusal::AlreadyDoneToday => "already_done_today",
            Refusal::UnknownMember => "unknown_member",
            Refusal::UnknownItem => "unknown_item",
            Refusal::UnknownEntry => "unknown_entry",
            Refusal::NotUndoable => "not_undoable",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Refusal::AlreadyDoneToday => "Already done today ✓",
            Refusal::UnknownMember => "That person isn't on the board",
            Refusal::UnknownItem => "That isn't on the board any more",
            Refusal::UnknownEntry => "That was already undone",
            Refusal::NotUndoable => "Only today's entries can be undone",
        }
    }
}

#[derive(Debug)]
pub enum CommandError {
    Refused(Refusal),
    Store(store::Error),
}

impl From<store::Error> for CommandError {
    fn from(e: store::Error) -> Self {
        CommandError::Store(e)
    }
}

/// A log entry that was just written.
#[derive(Clone, Debug)]
pub struct Earned {
    pub entry_id: i64,
    pub member_id: i64,
    pub item_id: i64,
    pub face: String,
    pub picture: String,
    pub amount: i64,
}

/// A family member did a chore (daily or occasional): earn its pebbles, at most once per local day.
pub async fn earn_chore(
    store: &Store,
    now: Timestamp,
    time_zone: &TimeZone,
    member_id: i64,
    item_id: i64,
) -> Result<Earned, CommandError> {
    let today = local_day(now, time_zone).to_string();
    let created_at = now.to_string();
    let outcome = store
        .call(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let face: Option<String> = tx
                .query_row(
                    "SELECT face FROM family_member WHERE id = ?1 AND retired = 0",
                    [member_id],
                    |r| r.get(0),
                )
                .optional()?;
            let Some(face) = face else {
                return Ok(Err(Refusal::UnknownMember));
            };
            let item: Option<(String, String, i64)> = tx
                .query_row(
                    "SELECT name, picture, value FROM item
                     WHERE id = ?1 AND retired = 0 AND kind IN (?2, ?3)",
                    (
                        item_id,
                        ItemKind::DailyChore.as_str(),
                        ItemKind::OccasionalChore.as_str(),
                    ),
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?;
            let Some((name, picture, value)) = item else {
                return Ok(Err(Refusal::UnknownItem));
            };
            let done_today: bool = tx.query_row(
                "SELECT EXISTS (SELECT 1 FROM log_entry
                 WHERE member_id = ?1 AND item_id = ?2 AND entry_type = 'earn' AND local_day = ?3)",
                (member_id, item_id, &today),
                |r| r.get(0),
            )?;
            if done_today {
                return Ok(Err(Refusal::AlreadyDoneToday));
            }
            tx.execute(
                "INSERT INTO log_entry
                     (member_id, entry_type, item_id, name, picture, amount, local_day, created_at)
                 VALUES (?1, 'earn', ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    member_id,
                    item_id,
                    &name,
                    &picture,
                    value,
                    &today,
                    &created_at,
                ),
            )?;
            let entry_id = tx.last_insert_rowid();
            tx.commit()?;
            Ok(Ok(Earned {
                entry_id,
                member_id,
                item_id,
                face,
                picture,
                amount: value,
            }))
        })
        .await?;
    outcome.map_err(CommandError::Refused)
}

/// A log entry that was just undone (removed from the log).
#[derive(Clone, Debug)]
pub struct Undone {
    pub member_id: i64,
    /// The chore, good deed or reward it was for, if it came from the board.
    pub item_id: Option<i64>,
    pub face: String,
    pub picture: String,
    pub amount: i64,
}

/// Undo a log entry: delete it, but only on the local day it was made.
/// Allowed even if it leaves a pot below zero; the board simply shows the result.
pub async fn undo_entry(
    store: &Store,
    now: Timestamp,
    time_zone: &TimeZone,
    entry_id: i64,
) -> Result<Undone, CommandError> {
    let today = local_day(now, time_zone).to_string();
    let outcome = store
        .call(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let entry: Option<(String, Undone)> = tx
                .query_row(
                    "SELECT l.local_day, l.member_id, l.item_id, m.face, l.picture, l.amount
                     FROM log_entry l JOIN family_member m ON m.id = l.member_id
                     WHERE l.id = ?1",
                    [entry_id],
                    |r| {
                        Ok((
                            r.get(0)?,
                            Undone {
                                member_id: r.get(1)?,
                                item_id: r.get(2)?,
                                face: r.get(3)?,
                                picture: r.get(4)?,
                                amount: r.get(5)?,
                            },
                        ))
                    },
                )
                .optional()?;
            let Some((day, undone)) = entry else {
                return Ok(Err(Refusal::UnknownEntry));
            };
            if day != today {
                return Ok(Err(Refusal::NotUndoable));
            }
            tx.execute("DELETE FROM log_entry WHERE id = ?1", [entry_id])?;
            tx.commit()?;
            Ok(Ok(undone))
        })
        .await?;
    outcome.map_err(CommandError::Refused)
}
