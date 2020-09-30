use super::api;
use api::{StashAccount, StashEntries, StashEntry};

use rusqlite::{named_params, params, Connection};
use std::path::Path;

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(path: P) -> Option<Self> {
        let conn = Connection::open(path.as_ref()).ok()?;

        conn.execute(
            r#"CREATE TABLE IF NOT EXISTS entry (
            id TEXT PRIMARY KEY NOT NULL,
            time INTEGER NOT NULL,
            league TEXT NOT NULL,
            item TEXT NOT NULL,
            action TEXT NOT NULL,
            account_name TEXT NOT NULL,
            account_realm TEXT NOT NULL,
            guild INTEGER NOT NULL
        )"#,
            params![],
        )
        .ok();

        conn.execute(
            r#"CREATE TABLE IF NOT EXISTS hole (
            id TEXT PRIMARY KEY NOT NULL,
            FOREIGN KEY(id) REFERENCES entry(id)
        )"#,
            params![],
        )
        .ok()?;

        Some(Self {
            conn,
        })
    }

    pub fn begin_insert(&mut self) -> Option<InsertTransaction<'_>> {
        let txn = self.conn.transaction().ok()?;
        Some(InsertTransaction {
            txn,
        })
    }

    pub fn fetch(&mut self, guildid: i64, age_limit: Option<chrono::NaiveDateTime>, count_limit: Option<i64>) -> rusqlite::Result<StashEntries> {
        let sql = r#"SELECT id,time,league,item,action,account_name,account_realm FROM entry
        WHERE guild = :guild ORDER BY CAST(id AS INTEGER) DESC"#;
        let mut stmt = self.conn.prepare(&sql)?;

        let mut entries = vec![];
        for entry in stmt.query_map_named(named_params! {
            ":guild": guildid,
        }, |row| {
            let time: i64 = row.get(1)?;
            Ok(StashEntry {
                id: row.get(0)?,
                time: time as u64,
                league: row.get(2)?,
                item: row.get(3)?,
                action: row.get(4)?,
                account: StashAccount {
                    name: row.get(5)?,
                    realm: row.get(6)?,
                }
            })
        })? {
            let e = entry?;
            if let Some(count) = count_limit {
                if entries.len() == count as usize {
                    break;
                }
            }
            if let Some(age) = &age_limit {
                if (e.time as i64) < age.timestamp() {
                    break;
                }
            }
            entries.push(e);
        }

        Ok(StashEntries {
            entries, 
        })
    }
}

pub struct InsertTransaction<'conn> {
    txn: rusqlite::Transaction<'conn>,
}

impl InsertTransaction<'_> {
    pub fn insert(&mut self, guildid: i64, entries: &[StashEntry]) -> rusqlite::Result<usize> {
        let mut added = 0;
        for entry in entries {
            added += self.txn.execute_named(
                r#"INSERT OR IGNORE INTO entry VALUES (:id,:time,:league,:item,:action,:account_name,:account_realm,:guild)"#,
                named_params! {
                    ":id": &entry.id,
                    ":time": entry.time as i64,
                    ":league": &entry.league,
                    ":item": &entry.item,
                    ":action": &entry.action,
                    ":account_name": &entry.account.name,
                    ":account_realm": &entry.account.realm,
                    ":guild": guildid,
                },
            )?;
        }
        Ok(added)
    }

    pub fn commit(self) -> Option<()> {
        self.txn.commit().ok()
    }
}