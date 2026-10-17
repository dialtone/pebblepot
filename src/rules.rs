//! The pebble rules: every command that changes the log, each in one immediate transaction.

use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use rusqlite::{OptionalExtension, TransactionBehavior};

use crate::clock::local_day;
use crate::model::{ItemKind, Role};
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
    /// The same family member earned the same good deed moments ago: a double-tap.
    RepeatTap,
    /// The reward costs more than the pot holds: an ordinary redeem never takes a pot below zero.
    NotEnoughPebbles,
    /// Save for this was tapped while already saving for a goal: one goal at a time.
    GoalAlreadyActive,
    /// Save for this on a reward the pot already covers: just redeem it.
    Affordable,
    /// Give up goal or Got it with no active goal.
    NoGoal,
    /// Got it before the pot is back to zero.
    GoalNotReady,
    /// Something else needs a name.
    NameNeeded,
    /// Something else's pebble value is outside `SOMETHING_ELSE_VALUES`.
    BadValue,
    /// A reward's price is outside `REWARD_PRICES`.
    BadPrice,
    /// A family member's colour isn't one of the `#rrggbb` colours the sheet offers.
    BadColour,
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
            Refusal::RepeatTap => "repeat_tap",
            Refusal::NotEnoughPebbles => "not_enough_pebbles",
            Refusal::GoalAlreadyActive => "goal_already_active",
            Refusal::Affordable => "affordable",
            Refusal::NoGoal => "no_goal",
            Refusal::GoalNotReady => "goal_not_ready",
            Refusal::NameNeeded => "name_needed",
            Refusal::BadValue => "bad_value",
            Refusal::BadPrice => "bad_price",
            Refusal::BadColour => "bad_colour",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Refusal::AlreadyDoneToday => "Already done today ✓",
            Refusal::UnknownMember => "That person isn't on the board",
            Refusal::UnknownItem => "That isn't on the board any more",
            Refusal::UnknownEntry => "That was already undone",
            Refusal::NotUndoable => "Only today's entries can be undone",
            Refusal::RepeatTap => "Already counted 👍",
            Refusal::NotEnoughPebbles => "Not enough pebbles yet 🔒",
            Refusal::GoalAlreadyActive => "Already saving for a goal ⭐",
            Refusal::Affordable => "Enough pebbles already: just tap it 🎁",
            Refusal::NoGoal => "Not saving for a goal",
            Refusal::GoalNotReady => "Not there yet ⭐",
            Refusal::NameNeeded => "Give it a name first ✏️",
            Refusal::BadValue => "Pick 1 to 100 pebbles",
            Refusal::BadPrice => "Pick a price from 1 to 1000 pebbles",
            Refusal::BadColour => "Pick a colour 🎨",
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

/// An identical good-deed earn this close to the previous one is a double-tap, not a second deed.
pub const REPEAT_TAP_WINDOW: SignedDuration = SignedDuration::from_secs(2);

/// A family member did a good deed: earn its pebbles. Good deeds repeat freely during the day,
/// but the same person earning the same good deed within `REPEAT_TAP_WINDOW` (inclusive) of
/// their previous earn of it is refused as a repeat tap.
pub async fn earn_good_deed(
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
                     WHERE id = ?1 AND retired = 0 AND kind = ?2",
                    (item_id, ItemKind::GoodDeed.as_str()),
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?;
            let Some((name, picture, value)) = item else {
                return Ok(Err(Refusal::UnknownItem));
            };
            let previous: Option<String> = tx
                .query_row(
                    "SELECT created_at FROM log_entry
                     WHERE member_id = ?1 AND item_id = ?2 AND entry_type = 'earn'
                     ORDER BY id DESC LIMIT 1",
                    (member_id, item_id),
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(previous) = previous {
                let previous: Timestamp = previous.parse()?;
                // Either way round, so a clock stepped backwards can't double-count either.
                if now.duration_since(previous).abs() <= REPEAT_TAP_WINDOW {
                    return Ok(Err(Refusal::RepeatTap));
                }
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
            undo_goal_effects(&tx, entry_id)?;
            tx.execute("DELETE FROM log_entry WHERE id = ?1", [entry_id])?;
            tx.commit()?;
            Ok(Ok(undone))
        })
        .await?;
    outcome.map_err(CommandError::Refused)
}

/// A redeem log entry that was just written.
#[derive(Clone, Debug)]
pub struct Redeemed {
    pub entry_id: i64,
    pub member_id: i64,
    pub item_id: i64,
    pub face: String,
    pub picture: String,
    /// Negative: the reward's price, taken from the pot.
    pub amount: i64,
}

/// A family member redeems a reward: write a redeem entry of −price, allowed only if price ≤ pot,
/// so an ordinary redeem never takes a pot below zero. While saving for a goal, only what's saved
/// can be spent: price ≤ saved (pot + goal price), so the pot never drops below −goal price.
pub async fn redeem(
    store: &Store,
    now: Timestamp,
    time_zone: &TimeZone,
    member_id: i64,
    item_id: i64,
) -> Result<Redeemed, CommandError> {
    let today = local_day(now, time_zone).to_string();
    let created_at = now.to_string();
    let outcome = store
        .call(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let member: Option<(String, i64)> = tx
                .query_row(
                    "SELECT m.face,
                            (SELECT COALESCE(SUM(amount), 0) FROM log_entry WHERE member_id = m.id)
                     FROM family_member m WHERE m.id = ?1 AND m.retired = 0",
                    [member_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            let Some((face, pot)) = member else {
                return Ok(Err(Refusal::UnknownMember));
            };
            let reward: Option<(String, String, i64)> = tx
                .query_row(
                    "SELECT name, picture, value FROM item
                     WHERE id = ?1 AND retired = 0 AND kind = ?2",
                    (item_id, ItemKind::Reward.as_str()),
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?;
            let Some((name, picture, price)) = reward else {
                return Ok(Err(Refusal::UnknownItem));
            };
            // With an active goal, only what's saved can be spent: saved = pot + goal price.
            let spendable = pot + active_goal_price(&tx, member_id)?.unwrap_or(0);
            if price > spendable {
                return Ok(Err(Refusal::NotEnoughPebbles));
            }
            tx.execute(
                "INSERT INTO log_entry
                     (member_id, entry_type, item_id, name, picture, amount, local_day, created_at)
                 VALUES (?1, 'redeem', ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    member_id,
                    item_id,
                    &name,
                    &picture,
                    -price,
                    &today,
                    &created_at,
                ),
            )?;
            let entry_id = tx.last_insert_rowid();
            tx.commit()?;
            Ok(Ok(Redeemed {
                entry_id,
                member_id,
                item_id,
                face,
                picture,
                amount: -price,
            }))
        })
        .await?;
    outcome.map_err(CommandError::Refused)
}

/// The price of `member_id`'s active goal, if they're saving for one.
fn active_goal_price(tx: &rusqlite::Transaction, member_id: i64) -> rusqlite::Result<Option<i64>> {
    tx.query_row(
        "SELECT price FROM goal WHERE member_id = ?1 AND status = 'active'",
        [member_id],
        |r| r.get(0),
    )
    .optional()
}

/// Before log entry `entry_id` is undone: undoing a goal's "goal set" entry deletes the goal
/// entirely (with its refund entry, if it was given up, so no free pebbles are left behind).
/// Undoing a "goal refund" entry takes the given-up goal back up, unless another goal is active.
fn undo_goal_effects(tx: &rusqlite::Transaction, entry_id: i64) -> rusqlite::Result<()> {
    let set: Option<(i64, Option<i64>)> = tx
        .query_row(
            "SELECT id, refund_entry_id FROM goal WHERE set_entry_id = ?1",
            [entry_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((goal_id, refund)) = set {
        tx.execute("DELETE FROM goal WHERE id = ?1", [goal_id])?;
        if let Some(refund) = refund {
            tx.execute("DELETE FROM log_entry WHERE id = ?1", [refund])?;
        }
        return Ok(());
    }
    tx.execute(
        "UPDATE goal SET status = 'active', refund_entry_id = NULL
         WHERE refund_entry_id = ?1
           AND NOT EXISTS (SELECT 1 FROM goal g WHERE g.member_id = goal.member_id AND g.status = 'active')",
        [entry_id],
    )?;
    // Still given up (another goal is active): just unlink the entry being deleted.
    tx.execute(
        "UPDATE goal SET refund_entry_id = NULL WHERE refund_entry_id = ?1",
        [entry_id],
    )?;
    Ok(())
}

/// A goal command that went through.
#[derive(Clone, Debug)]
pub struct GoalChanged {
    pub member_id: i64,
    pub face: String,
    pub picture: String,
    /// The log entry it wrote (offered for undo), if any: −price for a goal set, +price for a
    /// refund. Got it writes nothing.
    pub entry_id: Option<i64>,
    pub amount: i64,
}

/// ⭐ Save for this: redeem a reward the family member can't afford yet as their goal. Allowed only
/// if the price is above their pot and they have no active goal. Writes a "goal set" entry of
/// −price, so the pot goes below zero by the shortfall, and creates the goal.
pub async fn set_goal(
    store: &Store,
    now: Timestamp,
    time_zone: &TimeZone,
    member_id: i64,
    item_id: i64,
) -> Result<GoalChanged, CommandError> {
    let today = local_day(now, time_zone).to_string();
    let created_at = now.to_string();
    let outcome = store
        .call(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let Some((face, pot)) = member_and_pot(&tx, member_id)? else {
                return Ok(Err(Refusal::UnknownMember));
            };
            let reward: Option<(String, String, i64)> = tx
                .query_row(
                    "SELECT name, picture, value FROM item
                     WHERE id = ?1 AND retired = 0 AND kind = ?2",
                    (item_id, ItemKind::Reward.as_str()),
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?;
            let Some((name, picture, price)) = reward else {
                return Ok(Err(Refusal::UnknownItem));
            };
            if active_goal_price(&tx, member_id)?.is_some() {
                return Ok(Err(Refusal::GoalAlreadyActive));
            }
            if price <= pot {
                return Ok(Err(Refusal::Affordable));
            }
            tx.execute(
                "INSERT INTO log_entry
                     (member_id, entry_type, item_id, name, picture, amount, local_day, created_at)
                 VALUES (?1, 'goal_set', ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    member_id,
                    item_id,
                    &name,
                    &picture,
                    -price,
                    &today,
                    &created_at,
                ),
            )?;
            let entry_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO goal (member_id, set_entry_id, item_id, name, picture, price, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active')",
                (member_id, entry_id, item_id, &name, &picture, price),
            )?;
            tx.commit()?;
            Ok(Ok(GoalChanged {
                member_id,
                face,
                picture,
                entry_id: Some(entry_id),
                amount: -price,
            }))
        })
        .await?;
    outcome.map_err(CommandError::Refused)
}

/// Give up goal: write a "goal refund" entry of +price and mark the goal given up. The goal-set
/// entry stays in the log, so history is never rewritten.
pub async fn give_up_goal(
    store: &Store,
    now: Timestamp,
    time_zone: &TimeZone,
    member_id: i64,
) -> Result<GoalChanged, CommandError> {
    let today = local_day(now, time_zone).to_string();
    let created_at = now.to_string();
    let outcome = store
        .call(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let Some((face, _pot)) = member_and_pot(&tx, member_id)? else {
                return Ok(Err(Refusal::UnknownMember));
            };
            let goal: Option<(i64, Option<i64>, String, String, i64)> = tx
                .query_row(
                    "SELECT id, item_id, name, picture, price FROM goal
                     WHERE member_id = ?1 AND status = 'active'",
                    [member_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )
                .optional()?;
            let Some((goal_id, item_id, name, picture, price)) = goal else {
                return Ok(Err(Refusal::NoGoal));
            };
            tx.execute(
                "INSERT INTO log_entry
                     (member_id, entry_type, item_id, name, picture, amount, local_day, created_at)
                 VALUES (?1, 'goal_refund', ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    member_id,
                    item_id,
                    &name,
                    &picture,
                    price,
                    &today,
                    &created_at,
                ),
            )?;
            let entry_id = tx.last_insert_rowid();
            tx.execute(
                "UPDATE goal SET status = 'given_up', refund_entry_id = ?1 WHERE id = ?2",
                (entry_id, goal_id),
            )?;
            tx.commit()?;
            Ok(Ok(GoalChanged {
                member_id,
                face,
                picture,
                entry_id: Some(entry_id),
                amount: price,
            }))
        })
        .await?;
    outcome.map_err(CommandError::Refused)
}

/// Got it: the goal reward was handed over. Allowed only when the goal is active and the pot is
/// back to zero or more (Ready). Writes nothing to the log; the pot shows pebbles again.
pub async fn hand_over_goal(store: &Store, member_id: i64) -> Result<GoalChanged, CommandError> {
    let outcome = store
        .call(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let Some((face, pot)) = member_and_pot(&tx, member_id)? else {
                return Ok(Err(Refusal::UnknownMember));
            };
            let goal: Option<(i64, String)> = tx
                .query_row(
                    "SELECT id, picture FROM goal WHERE member_id = ?1 AND status = 'active'",
                    [member_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            let Some((goal_id, picture)) = goal else {
                return Ok(Err(Refusal::NoGoal));
            };
            if pot < 0 {
                return Ok(Err(Refusal::GoalNotReady));
            }
            tx.execute(
                "UPDATE goal SET status = 'handed_over' WHERE id = ?1",
                [goal_id],
            )?;
            tx.commit()?;
            Ok(Ok(GoalChanged {
                member_id,
                face,
                picture,
                entry_id: None,
                amount: 0,
            }))
        })
        .await?;
    outcome.map_err(CommandError::Refused)
}

/// An active family member's face and pot, summed inside the caller's transaction.
fn member_and_pot(
    tx: &rusqlite::Transaction,
    member_id: i64,
) -> rusqlite::Result<Option<(String, i64)>> {
    tx.query_row(
        "SELECT m.face,
                (SELECT COALESCE(SUM(amount), 0) FROM log_entry WHERE member_id = m.id)
         FROM family_member m WHERE m.id = ?1 AND m.retired = 0",
        [member_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
}

/// A new something else starts at this many pebbles.
pub const SOMETHING_ELSE_DEFAULT_VALUE: i64 = 1;
/// The pebble values Something else accepts: pebbles are only ever gained, and the cap stops a typo
/// from filling a pot.
pub const SOMETHING_ELSE_VALUES: std::ops::RangeInclusive<i64> = 1..=100;
/// The picture used when none is given.
pub const SOMETHING_ELSE_PICTURE: &str = "✨";

/// What ✏️ Something else was filled in with.
#[derive(Clone, Debug)]
pub struct SomethingElse {
    /// The section it was added from: a daily chore, occasional chore or good deed.
    pub kind: ItemKind,
    pub name: String,
    pub picture: String,
    pub value: i64,
    /// On: create the item in that section, then earn it. Off: a one-off log entry with no item.
    pub keep_on_board: bool,
}

/// A something else that was just earned.
#[derive(Clone, Debug)]
pub struct EarnedSomethingElse {
    pub entry_id: i64,
    pub member_id: i64,
    /// The new chore or good deed, if it was kept on the board.
    pub item_id: Option<i64>,
    pub face: String,
    pub picture: String,
    pub amount: i64,
}

/// A family member did something that isn't on the board yet: earn it in one step. Kept on the
/// board, it first becomes a new item at the end of its section (and then follows that section's
/// rules); otherwise only a one-off log entry is written. The same person earning something of the
/// same name within `REPEAT_TAP_WINDOW` is refused as a repeat tap, so a double-tapped
/// "Give pebbles" neither earns twice nor adds two rows.
pub async fn earn_something_else(
    store: &Store,
    now: Timestamp,
    time_zone: &TimeZone,
    member_id: i64,
    what: SomethingElse,
) -> Result<EarnedSomethingElse, CommandError> {
    if !matches!(
        what.kind,
        ItemKind::DailyChore | ItemKind::OccasionalChore | ItemKind::GoodDeed
    ) {
        return Err(CommandError::Refused(Refusal::UnknownItem));
    }
    let name = what.name.trim().to_string();
    if name.is_empty() {
        return Err(CommandError::Refused(Refusal::NameNeeded));
    }
    if !SOMETHING_ELSE_VALUES.contains(&what.value) {
        return Err(CommandError::Refused(Refusal::BadValue));
    }
    let picture = match what.picture.trim() {
        "" => SOMETHING_ELSE_PICTURE.to_string(),
        picture => picture.to_string(),
    };
    let (kind, value, keep) = (what.kind, what.value, what.keep_on_board);
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
            let previous: Option<String> = tx
                .query_row(
                    "SELECT created_at FROM log_entry
                     WHERE member_id = ?1 AND name = ?2 AND entry_type = 'earn'
                     ORDER BY id DESC LIMIT 1",
                    (member_id, &name),
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(previous) = previous {
                let previous: Timestamp = previous.parse()?;
                if now.duration_since(previous).abs() <= REPEAT_TAP_WINDOW {
                    return Ok(Err(Refusal::RepeatTap));
                }
            }
            let item_id = if keep {
                tx.execute(
                    "INSERT INTO item (kind, name, picture, value, position)
                     SELECT ?1, ?2, ?3, ?4, COALESCE(MAX(position), 0) + 1 FROM item WHERE kind = ?1",
                    (kind.as_str(), &name, &picture, value),
                )?;
                Some(tx.last_insert_rowid())
            } else {
                None
            };
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
            Ok(Ok(EarnedSomethingElse {
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

// ✏️ Edit mode. Items are changed in place and retired rather than deleted. Log entries and goals
// keep their own copies of the name, picture and amount, so none of these touch a pot or the past.

/// What an item is changed to with the ✏️ pencil.
#[derive(Clone, Debug)]
pub struct ItemChange {
    pub name: String,
    /// Blank becomes ✨, as for Something else.
    pub picture: String,
    /// Pebbles earned, within `SOMETHING_ELSE_VALUES`, or a reward's price, within
    /// `REWARD_PRICES`.
    pub value: i64,
    /// The item's kind afterwards. Only a chore can change kind: daily ⇄ occasional.
    pub kind: ItemKind,
}

/// An item as it was just changed or retired.
#[derive(Clone, Debug)]
pub struct EditedItem {
    pub item_id: i64,
    pub name: String,
    pub picture: String,
    /// Its kind before the change (a daily ⇄ occasional switch moves it between sections).
    pub was: ItemKind,
    pub kind: ItemKind,
}

/// A reward's price: a new one starts at `NEW_REWARD_PRICE`. Wider than `SOMETHING_ELSE_VALUES`,
/// since big rewards are saved up for (the starting Big toy is 100), but capped so a typo can't
/// put a reward out of reach for good.
pub const REWARD_PRICES: std::ops::RangeInclusive<i64> = 1..=1000;
pub const NEW_REWARD_PRICE: i64 = 1;
/// The picture a reward gets when none is given.
pub const REWARD_PICTURE: &str = "🎁";

/// A change's trimmed name and its picture (a blank one becomes the kind's default), or why it's
/// refused: no name, or a value outside the kind's range.
fn checked(change: &ItemChange) -> Result<(String, String), Refusal> {
    let name = change.name.trim().to_string();
    if name.is_empty() {
        return Err(Refusal::NameNeeded);
    }
    let (values, refusal, default_picture) = if change.kind == ItemKind::Reward {
        (REWARD_PRICES, Refusal::BadPrice, REWARD_PICTURE)
    } else {
        (
            SOMETHING_ELSE_VALUES,
            Refusal::BadValue,
            SOMETHING_ELSE_PICTURE,
        )
    };
    if !values.contains(&change.value) {
        return Err(refusal);
    }
    let picture = match change.picture.trim() {
        "" => default_picture.to_string(),
        picture => picture.to_string(),
    };
    Ok((name, picture))
}

/// ＋ Add a new item of `change.kind` (a reward, from the Spend sheet) at the end of its list.
pub async fn create_item(store: &Store, change: ItemChange) -> Result<EditedItem, CommandError> {
    let (name, picture) = checked(&change).map_err(CommandError::Refused)?;
    let (value, kind) = (change.value, change.kind);
    Ok(store
        .call(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute(
                "INSERT INTO item (kind, name, picture, value, position)
                 SELECT ?1, ?2, ?3, ?4, COALESCE(MAX(position), 0) + 1 FROM item WHERE kind = ?1",
                (kind.as_str(), &name, &picture, value),
            )?;
            let item_id = tx.last_insert_rowid();
            tx.commit()?;
            Ok(EditedItem {
                item_id,
                name,
                picture,
                was: kind,
                kind,
            })
        })
        .await?)
}

fn is_chore(kind: ItemKind) -> bool {
    matches!(kind, ItemKind::DailyChore | ItemKind::OccasionalChore)
}

fn kind_from_str(kind: &str) -> Option<ItemKind> {
    [
        ItemKind::DailyChore,
        ItemKind::OccasionalChore,
        ItemKind::GoodDeed,
        ItemKind::Reward,
    ]
    .into_iter()
    .find(|k| k.as_str() == kind)
}

/// The kind of an active (not retired) item, read inside the caller's transaction.
fn active_item_kind(
    tx: &rusqlite::Transaction,
    item_id: i64,
) -> rusqlite::Result<Option<ItemKind>> {
    let kind: Option<String> = tx
        .query_row(
            "SELECT kind FROM item WHERE id = ?1 AND retired = 0",
            [item_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(kind.as_deref().and_then(kind_from_str))
}

/// ✏️ Change an item's name, picture and value, and switch a chore between daily and occasional.
/// A chore that switches goes to the end of its new section. Today's ✓s and the once-a-day rule
/// follow the item, since they key on it. Log entries already written keep their own copies.
pub async fn edit_item(
    store: &Store,
    item_id: i64,
    change: ItemChange,
) -> Result<EditedItem, CommandError> {
    let (name, picture) = checked(&change).map_err(CommandError::Refused)?;
    let (value, kind) = (change.value, change.kind);
    let outcome = store
        .call(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let Some(was) = active_item_kind(&tx, item_id)? else {
                return Ok(Err(Refusal::UnknownItem));
            };
            if was != kind && !(is_chore(was) && is_chore(kind)) {
                return Ok(Err(Refusal::UnknownItem));
            }
            if was == kind {
                tx.execute(
                    "UPDATE item SET name = ?1, picture = ?2, value = ?3 WHERE id = ?4",
                    (&name, &picture, value, item_id),
                )?;
            } else {
                tx.execute(
                    "UPDATE item SET name = ?1, picture = ?2, value = ?3, kind = ?4,
                         position = (SELECT COALESCE(MAX(position), 0) + 1 FROM item WHERE kind = ?4)
                     WHERE id = ?5",
                    (&name, &picture, value, kind.as_str(), item_id),
                )?;
            }
            tx.commit()?;
            Ok(Ok(EditedItem {
                item_id,
                name,
                picture,
                was,
                kind,
            }))
        })
        .await?;
    outcome.map_err(CommandError::Refused)
}

/// 🗑 Retire an item: it leaves the board (and Spend) for good. Nothing is deleted, so its log
/// entries, and any goal set on it, stay as they were.
pub async fn retire_item(store: &Store, item_id: i64) -> Result<EditedItem, CommandError> {
    let outcome = store
        .call(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let item: Option<(String, String, String)> = tx
                .query_row(
                    "SELECT name, picture, kind FROM item WHERE id = ?1 AND retired = 0",
                    [item_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?;
            let Some((name, picture, kind)) = item else {
                return Ok(Err(Refusal::UnknownItem));
            };
            let kind = kind_from_str(&kind).ok_or("unknown item kind")?;
            tx.execute("UPDATE item SET retired = 1 WHERE id = ?1", [item_id])?;
            tx.commit()?;
            Ok(Ok(EditedItem {
                item_id,
                name,
                picture,
                was: kind,
                kind,
            }))
        })
        .await?;
    outcome.map_err(CommandError::Refused)
}

/// Drop a dragged row: `order` is the new order of some or all of `kind`'s active items (a
/// searched list shows only some). Those items swap among the places they already hold, and every
/// other item keeps its place. Anything retired, of another kind, unknown or repeated is refused.
pub async fn reorder_items(
    store: &Store,
    kind: ItemKind,
    order: Vec<i64>,
) -> Result<(), CommandError> {
    let outcome = store
        .call(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let current = tx
                .prepare(
                    "SELECT id FROM item WHERE kind = ?1 AND retired = 0 ORDER BY position, id",
                )?
                .query_map([kind.as_str()], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let moved: std::collections::HashSet<i64> = order.iter().copied().collect();
            if moved.len() != order.len() || !moved.iter().all(|id| current.contains(id)) {
                return Ok(Err(Refusal::UnknownItem));
            }
            let mut dropped = order.iter();
            for (position, id) in current.iter().enumerate() {
                let id = if moved.contains(id) {
                    *dropped.next().expect("as many places as moved items")
                } else {
                    *id
                };
                tx.execute(
                    "UPDATE item SET position = ?1 WHERE id = ?2",
                    (position as i64 + 1, id),
                )?;
            }
            tx.commit()?;
            Ok(Ok(()))
        })
        .await?;
    outcome.map_err(CommandError::Refused)
}

// ✏️ Family members: added, changed and retired from the pots in edit mode. A retired family
// member leaves the board, Spend and every row, and their log entries (and any goal) stay put.

/// The face a family member gets when none is given.
pub const MEMBER_FACE: &str = "🙂";
/// The colours the family member sheet offers. Any `#rrggbb` colour is accepted.
pub const MEMBER_COLOURS: [&str; 10] = [
    "#f59e0b", "#3b82f6", "#db2777", "#16a34a", "#8b5cf6", "#ef4444", "#0ea5e9", "#14b8a6",
    "#f97316", "#6b7280",
];

/// What a family member is added as, or changed to, with the ✏️ sheet.
#[derive(Clone, Debug)]
pub struct MemberChange {
    pub name: String,
    /// Blank becomes `MEMBER_FACE`.
    pub face: String,
    /// `#rrggbb`.
    pub colour: String,
}

/// A family member as they were just added, changed or retired.
#[derive(Clone, Debug)]
pub struct EditedMember {
    pub member_id: i64,
    pub name: String,
    pub face: String,
}

/// A change's trimmed name, its face (blank becomes `MEMBER_FACE`) and its lower-case colour, or
/// why it's refused.
fn checked_member(change: &MemberChange) -> Result<(String, String, String), Refusal> {
    let name = change.name.trim().to_string();
    if name.is_empty() {
        return Err(Refusal::NameNeeded);
    }
    let colour = change.colour.trim().to_ascii_lowercase();
    let hex = colour.strip_prefix('#').unwrap_or("");
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Refusal::BadColour);
    }
    let face = match change.face.trim() {
        "" => MEMBER_FACE.to_string(),
        face => face.to_string(),
    };
    Ok((name, face, colour))
}

/// ＋ Add someone: a new kid or parent at the end of the pots, starting at 0 pebbles.
pub async fn create_member(
    store: &Store,
    role: Role,
    change: MemberChange,
) -> Result<EditedMember, CommandError> {
    let (name, face, colour) = checked_member(&change).map_err(CommandError::Refused)?;
    Ok(store
        .call(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute(
                "INSERT INTO family_member (name, face, colour, role, position)
                 SELECT ?1, ?2, ?3, ?4, COALESCE(MAX(position), 0) + 1 FROM family_member",
                (&name, &face, &colour, role.as_str()),
            )?;
            let member_id = tx.last_insert_rowid();
            tx.commit()?;
            Ok(EditedMember {
                member_id,
                name,
                face,
            })
        })
        .await?)
}

/// ✏️ Rename, re-face or re-colour a family member. Their log entries keep their own copies of
/// what each entry was for, and the pot is still what they add up to.
pub async fn edit_member(
    store: &Store,
    member_id: i64,
    change: MemberChange,
) -> Result<EditedMember, CommandError> {
    let (name, face, colour) = checked_member(&change).map_err(CommandError::Refused)?;
    let outcome = store
        .call(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let changed = tx.execute(
                "UPDATE family_member SET name = ?1, face = ?2, colour = ?3
                 WHERE id = ?4 AND retired = 0",
                (&name, &face, &colour, member_id),
            )?;
            if changed == 0 {
                return Ok(Err(Refusal::UnknownMember));
            }
            tx.commit()?;
            Ok(Ok(EditedMember {
                member_id,
                name,
                face,
            }))
        })
        .await?;
    outcome.map_err(CommandError::Refused)
}

/// 🗑 Retire a family member: their pot card and faces leave the board for good. Nothing is
/// deleted, so their log entries, and any goal they were saving for, stay as they were.
pub async fn retire_member(store: &Store, member_id: i64) -> Result<EditedMember, CommandError> {
    let outcome = store
        .call(move |conn| {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let member: Option<(String, String)> = tx
                .query_row(
                    "SELECT name, face FROM family_member WHERE id = ?1 AND retired = 0",
                    [member_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            let Some((name, face)) = member else {
                return Ok(Err(Refusal::UnknownMember));
            };
            tx.execute(
                "UPDATE family_member SET retired = 1 WHERE id = ?1",
                [member_id],
            )?;
            tx.commit()?;
            Ok(Ok(EditedMember {
                member_id,
                name,
                face,
            }))
        })
        .await?;
    outcome.map_err(CommandError::Refused)
}
