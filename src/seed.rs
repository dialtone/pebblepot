//! The starting content, loaded once on first start (see .scratch/pebblepot-v1/starting-content.md).
//! Family member names are placeholders, renamed in the app: real names stay out of this repo.

use crate::model::{ItemKind, Role};
use crate::store::{Result, Store};

/// (name, face, colour, role)
const FAMILY: &[(&str, &str, &str, Role)] = &[
    ("Kid 1", "🦊", "#f59e0b", Role::Kid),
    ("Kid 2", "🐻", "#3b82f6", Role::Kid),
    ("Mom", "🦉", "#db2777", Role::Parent),
    ("Dad", "🐢", "#16a34a", Role::Parent),
];

/// (picture, name, pebbles)
const DAILY_CHORES: &[(&str, &str, i64)] = &[
    ("🛏️", "Make bed", 1),
    ("👕", "Get dressed", 1),
    ("🪥", "Brush teeth (morning)", 1),
    ("🌙", "Brush teeth (bedtime)", 1),
    ("🍽️", "Clear my plate", 1),
    ("🧸", "Tidy toys", 2),
    ("🧺", "Dirty clothes in the hamper", 1),
    ("🎒", "Unpack my bag", 1),
    ("🛁", "Bath or shower time, no fuss", 1),
];

const GOOD_DEEDS: &[(&str, &str, i64)] = &[
    ("🤝", "Shared nicely", 2),
    ("🫶", "Helped my brother or sister", 2),
    ("🙌", "Helped without being asked", 3),
    ("👂", "Listened the first time", 1),
    ("🙏", "Said please and thank you", 1),
    ("💬", "Said sorry", 1),
    ("💛", "Kind words", 1),
    ("🧘", "Stayed calm when upset", 2),
    ("🦁", "Was brave", 2),
    ("🥦", "Tried a new food", 2),
    ("🧩", "Played well together", 2),
];

const OCCASIONAL_CHORES: &[(&str, &str, i64)] = &[
    ("🍴", "Set the table", 2),
    ("🧽", "Wipe the table", 2),
    ("🍳", "Help cook", 3),
    ("🍽️", "Unload the dishwasher", 3),
    ("🧦", "Match the socks", 2),
    ("👚", "Fold laundry", 3),
    ("🗄️", "Put my clothes away", 2),
    ("🧹", "Sweep the floor", 3),
    ("🌀", "Vacuum a room", 3),
    ("🛏️", "Tidy my whole room", 4),
    ("🪴", "Water the plants", 2),
    ("♻️", "Sort the recycling", 2),
    ("🗑️", "Take out the trash", 2),
    ("👟", "Tidy the shoes", 1),
    ("🛍️", "Carry the shopping", 2),
    ("🍂", "Rake leaves", 4),
    ("🚗", "Help wash the car", 4),
    ("💌", "Make a card for someone", 2),
];

/// (picture, name, price)
const REWARDS: &[(&str, &str, i64)] = &[
    ("⭐", "Sticker", 5),
    ("📖", "Extra bedtime story", 8),
    ("🍦", "Pick dessert", 10),
    ("📺", "15 minutes extra screen time", 10),
    ("🍝", "Choose dinner", 15),
    ("🌜", "Stay up 15 minutes later", 15),
    ("🛝", "Trip to the park", 15),
    ("🧁", "Bake something together", 20),
    ("🎬", "Pick the family movie", 20),
    ("🍿", "Movie night with popcorn", 30),
    ("👫", "Playdate with a friend", 40),
    ("🪀", "Small toy", 40),
    ("🎡", "Day out", 60),
    ("🧸", "Big toy", 100),
    ("☕", "Breakfast in bed (grown-ups)", 20),
    ("😴", "Sleep in (grown-ups)", 30),
    ("🤫", "30 minutes of quiet (grown-ups)", 15),
];

/// Loads the starting content if the household has no family members yet.
/// Returns whether it seeded. Never runs again once anyone exists, so edits are never overwritten.
pub async fn seed_if_empty(store: &Store) -> Result<bool> {
    store
        .call(|conn| {
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let members: i64 = tx.query_row("SELECT COUNT(*) FROM family_member", [], |r| r.get(0))?;
            if members > 0 {
                return Ok(false);
            }
            for (position, (name, face, colour, role)) in FAMILY.iter().enumerate() {
                tx.execute(
                    "INSERT INTO family_member (name, face, colour, role, position) VALUES (?1, ?2, ?3, ?4, ?5)",
                    (name, face, colour, role.as_str(), position as i64),
                )?;
            }
            let lists = [
                (ItemKind::DailyChore, DAILY_CHORES),
                (ItemKind::GoodDeed, GOOD_DEEDS),
                (ItemKind::OccasionalChore, OCCASIONAL_CHORES),
                (ItemKind::Reward, REWARDS),
            ];
            for (kind, items) in lists {
                for (position, (picture, name, value)) in items.iter().enumerate() {
                    tx.execute(
                        "INSERT INTO item (kind, name, picture, value, position) VALUES (?1, ?2, ?3, ?4, ?5)",
                        (kind.as_str(), name, picture, value, position as i64),
                    )?;
                }
            }
            tx.commit()?;
            Ok(true)
        })
        .await
}
