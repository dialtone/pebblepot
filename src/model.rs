//! Domain vocabulary shared by the store, the seed and the board. See GLOSSARY.md.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Kid,
    Parent,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Kid => "kid",
            Role::Parent => "parent",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        [Role::Kid, Role::Parent]
            .into_iter()
            .find(|role| role.as_str() == key)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    DailyChore,
    OccasionalChore,
    GoodDeed,
    Reward,
}

impl ItemKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ItemKind::DailyChore => "daily_chore",
            ItemKind::OccasionalChore => "occasional_chore",
            ItemKind::GoodDeed => "good_deed",
            ItemKind::Reward => "reward",
        }
    }
}
