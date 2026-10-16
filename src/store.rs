//! The SQLite database: one connection on its own thread, WAL mode, migrations at start-up.

use std::path::Path;
use std::sync::LazyLock;

use rusqlite_migration::{M, Migrations};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub const DATABASE_FILE: &str = "pebblepot.sqlite3";

static MIGRATIONS: LazyLock<Migrations<'static>> = LazyLock::new(|| {
    Migrations::new(vec![
        M::up(include_str!("../migrations/0001_initial.sql")),
        M::up(include_str!("../migrations/0002_goal.sql")),
    ])
});

#[derive(Clone)]
pub struct Store {
    conn: tokio_rusqlite::Connection,
}

impl Store {
    /// Opens (creating if needed) the database in `data_dir` and brings it up to date.
    pub async fn open(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let conn = tokio_rusqlite::Connection::open(data_dir.join(DATABASE_FILE)).await?;
        let store = Store { conn };
        store
            .call(|conn| {
                conn.pragma_update(None, "journal_mode", "WAL")?;
                conn.pragma_update(None, "foreign_keys", "ON")?;
                MIGRATIONS.to_latest(conn)?;
                Ok(())
            })
            .await?;
        Ok(store)
    }

    /// Runs `f` on the connection's own thread.
    pub async fn call<R, F>(&self, f: F) -> Result<R>
    where
        R: Send + 'static,
        F: FnOnce(&mut rusqlite::Connection) -> Result<R> + Send + 'static,
    {
        self.conn.call(f).await.map_err(|e| match e {
            tokio_rusqlite::Error::Error(e) => e,
            other => other.to_string().into(),
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn migrations_are_valid() {
        super::MIGRATIONS.validate().unwrap();
    }
}
