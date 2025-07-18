use super::api;
use api::{StashAccount, StashEntries, StashEntry};

use rusqlite::{named_params, params, Connection};
use std::path::Path;

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(path: P) -> rusqlite::Result<Self> {
        let conn = Connection::open(path.as_ref())?;

        conn.execute(
            r#"CREATE TABLE IF NOT EXISTS entry (
            id TEXT PRIMARY KEY NOT NULL,
            time INTEGER NOT NULL,
            league TEXT NOT NULL,
            item TEXT NOT NULL,
            action TEXT NOT NULL,
            account_name TEXT NOT NULL,
            guild INTEGER NOT NULL,
            stash TEXT,
            x INTEGER,
            y INTEGER
        )"#,
            params![],
        )?;

        conn.execute(
            r#"CREATE TABLE IF NOT EXISTS hole (
            id TEXT PRIMARY KEY NOT NULL,
            FOREIGN KEY(id) REFERENCES entry(id)
        )"#,
            params![],
        )?;

        let _ = conn.execute(r#"ALTER TABLE entry ADD COLUMN stash TEXT"#, params![]);
        let _ = conn.execute(r#"ALTER TABLE entry ADD COLUMN x INTEGER"#, params![]);
        let _ = conn.execute(r#"ALTER TABLE entry ADD COLUMN y INTEGER"#, params![]);
        let _ = conn.execute(r#"ALTER TABLE entry DROP COLUMN realm"#, params![]);

        Ok(Self { conn })
    }

    pub fn begin_insert(&mut self) -> Option<InsertTransaction<'_>> {
        let txn = self.conn.transaction().ok()?;
        Some(InsertTransaction { txn })
    }

    pub fn fetch(
        &mut self,
        guildid: i64,
        age_limit: Option<chrono::NaiveDateTime>,
        count_limit: Option<i64>,
    ) -> rusqlite::Result<StashEntries> {
        let sql = r#"SELECT id,time,league,item,action,account_name,stash,x,y FROM entry
        WHERE guild = :guild ORDER BY CAST(id AS INTEGER) DESC"#;
        let mut stmt = self.conn.prepare(&sql)?;

        let mut entries = vec![];
        for entry in stmt.query_map(
            named_params! {
                ":guild": guildid,
            },
            |row| {
                let time: i64 = row.get(1)?;
                Ok(StashEntry {
                    id: row.get(0)?,
                    time: time as u64,
                    league: row.get(2)?,
                    item: row.get(3)?,
                    action: row.get(4)?,
                    account: StashAccount {
                        name: row.get(5)?,
                        realm: None,
                    },
                    stash: row.get(6)?,
                    x: row.get(7)?,
                    y: row.get(8)?,
                })
            },
        )? {
            let e = entry?;
            if let Some(count) = count_limit {
                if entries.len() == count as usize {
                    break;
                }
            }
            if let Some(age) = &age_limit {
                if (e.time as i64) < age.and_utc().timestamp() {
                    break;
                }
            }
            entries.push(e);
        }

        Ok(StashEntries { entries })
    }
}

pub struct InsertTransaction<'conn> {
    txn: rusqlite::Transaction<'conn>,
}

impl InsertTransaction<'_> {
    pub fn insert(&mut self, guildid: i64, entries: &[StashEntry]) -> rusqlite::Result<usize> {
        let mut added = 0;
        for entry in entries {
            added += self.txn.execute(
                r#"INSERT OR IGNORE INTO entry
                (id,time,league,item,action,account_name,guild,stash,x,y)
                VALUES
                (:id,:time,:league,:item,:action,:account_name,:guild,:stash,:x,:y)"#,
                named_params! {
                    ":id": &entry.id,
                    ":time": entry.time as i64,
                    ":league": &entry.league,
                    ":item": &entry.item,
                    ":action": &entry.action,
                    ":account_name": &entry.account.name,
                    ":guild": guildid,
                    ":stash": &entry.stash,
                    ":x": &entry.x,
                    ":y": &entry.y,
                },
            )?;
        }
        Ok(added)
    }

    pub fn commit(self) -> Option<()> {
        self.txn.commit().ok()
    }
}
