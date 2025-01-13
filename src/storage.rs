use rusqlite;
use rusqlite::{Connection, Result};
use std::fmt;
use std::error::Error;
use std::path::PathBuf;


const SCHEMA_VERSION: i32 = 1;

#[derive(Debug)]
pub struct SQLTimerRow {
    pub id: u64,
    pub task: String,
    pub start: u64,
    pub end: u64,
    pub idle: i64,
    pub status: u32
}

#[derive(Debug)]
pub enum StorageError {
    SchemaVersionError,
    CurrentTimerNotClosed,
    TimerDoesNotExists,
    NotSupportedValue,
    TimerHasBeenClosed
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StorageError::TimerDoesNotExists => write!(f, "Timer does not exist"),
            StorageError::SchemaVersionError => write!(f, "Version of db is no correct"),
            StorageError::CurrentTimerNotClosed => write!(f, "Current timer is not closed"),
            StorageError::NotSupportedValue => write!(f, "Not supported values"),
            StorageError::TimerHasBeenClosed => write!(f, "Timer has been closed"),
        }
    }
}

impl Error for StorageError {}

#[derive(Debug)]
pub struct Storage {
    conn: Connection,
    items: Vec<SQLTimerRow>
}

impl Storage {

    fn get_version(conn: &Connection) -> Option<i32> {
        match conn.query_row(
            "SELECT version FROM schema_version",
            [],
            | row | { Ok(row.get(0)?) }
        ) {
            Ok(ver) => ver,
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => panic!("Database error: {e}")
        }
    }

    pub fn from_memory() -> Result<Storage, Box<dyn Error>> {
        Self::new(None)
    }

    pub fn from_path(path: PathBuf) -> Result<Storage, Box<dyn Error>> {
        Self::new(Some(path))
    }

    pub fn new(path: Option<PathBuf>) -> Result<Storage, Box<dyn Error>> {
        let conn = if let Some(path) = path {
            Connection::open(path)?
        } else {
            Connection::open_in_memory()?
        };

        conn.execute("CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER
        )", [])?;
       
        match Storage::get_version(&conn) {
            Some(v) => {
                if v != SCHEMA_VERSION {
                    return Err(Box::new(StorageError::SchemaVersionError));
                }
            },
            None => {
                conn.execute(
                    "INSERT INTO schema_version (version) VALUES (?1)",
                    rusqlite::params![SCHEMA_VERSION]
                )?;
            }
        }

        conn.execute("CREATE TABLE IF NOT EXISTS timers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            start INTEGER,
            task STRING,
            end INTEGER,
            idle INTEGER,
            status INTEGER
        )", [])?;


        Ok(Storage {
            conn,
            items: Vec::new()
        })
    }

    pub fn insert_timer(&self, timer: &SQLTimerRow) -> Result<i64, Box<dyn Error>> {
        self.conn.execute("
            INSERT INTO timers
                (task, start, end, idle, status)
                VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            rusqlite::params![
                timer.task,
                timer.start,
                timer.end,
                timer.idle,
                timer.status
            ]
        )?;
        Ok(self.conn.last_insert_rowid())    
    }

    pub fn update_timer(&self, timer: SQLTimerRow) -> Result<(), Box<dyn Error>> {
        self.conn.execute("
            UPDATE timers SET
                task=?1, start=?2, end=?3, idle=?4, status=?5
            WHERE id=?6
            ",
            rusqlite::params![
                timer.task,
                timer.start,
                timer.end,
                timer.idle,
                timer.status,
                timer.id
            ]
        )?;
        Ok(())    
    }

    pub fn get_timer_by_id(&self, id: i64) -> Result<SQLTimerRow, Box<dyn Error>> {
        let q = "
            SELECT id, task, start, end, idle, status
            FROM timers
            WHERE id = ?1
        ";
        Ok(self.conn.query_row(q, rusqlite::params![id], | row | Ok(SQLTimerRow {
            id: row.get(0)?,
            task: row.get(1)?,
            start: row.get(2)?,
            end: row.get(3)?,
            idle: row.get(4)?,
            status: row.get(5)?
        }))?)
    }

    pub fn get_timers_by_status(
        &mut self,
        status: u32,
        limit: i32
    ) -> Result<&Vec<SQLTimerRow>, Box<dyn Error>> {
        let q = "
            SELECT id, task, start, end, idle, status
            FROM timers
            WHERE status = ?1
            ORDER BY id DESC
            LIMIT ?2
        ";
        let mut stmt = self.conn.prepare(q)?;
        let items = stmt.query_map(
            rusqlite::params![status, limit],
            | row | {
                Ok(SQLTimerRow{
                    id: row.get(0)?,
                    task: row.get(1)?,
                    start: row.get(2)?,
                    end: row.get(3)?,
                    idle: row.get(4)?,
                    status: row.get(5)?
                })
            }
        )?;
        self.items.clear();
        self.items.extend(items.filter_map(Result::ok));
        Ok(&self.items)
    }

    pub fn get_timers_by_date(
        &mut self,
        limit: i32,
        date_from: Option<u64>,
        date_to: Option<u64>
    ) -> Result<&Vec<SQLTimerRow>, Box<dyn Error>> {
        let query = "
            SELECT id, task, start, end, idle, status
            FROM timers
            WHERE
                (?1 is NULL OR start >= ?1)
                AND (?2 is NULL OR start < ?2)
            ORDER BY start DESC
            LIMIT ?3
        ";
        let mut stmt = self.conn.prepare(query)?;
        let items = stmt.query_map(
            rusqlite::params![date_from, date_to, limit],
            | row | {
                Ok(SQLTimerRow{
                    id: row.get(0)?,
                    task: row.get(1)?,
                    start: row.get(2)?,
                    end: row.get(3)?,
                    idle: row.get(4)?,
                    status: row.get(5)?
                })
            }
        )?;
        self.items.clear();
        self.items.extend(items.filter_map(Result::ok));
        Ok(&self.items)
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    fn str_to_time(time_s: String) -> u64 {
        DateTime::parse_from_str(
            format!("{} +0000", time_s).as_str(),
            "%Y-%m-%d %H:%M:%S %z"
        ).unwrap().timestamp() as u64
    }

    #[test]
    fn test_create_storage() {
        let storage = Storage::from_memory();
        match storage {
            Ok(_) => (),
            Err(e) => panic!("Error {e}")
        }
    }

    #[test]
    fn test_insert() {
        let row = SQLTimerRow {
            id: 0,
            task: "test".to_string(),
            start: str_to_time("2024-01-01 00:00:00".to_string()),
            end: str_to_time("2024-01-01 00:00:00".to_string()),
            idle: 0,
            status: 1
        };
        let storage = Storage::from_memory().expect("err");
        let id = storage.insert_timer(&row).expect("Problem");
        let mut timer = storage.get_timer_by_id(id).unwrap();
        assert_eq!(id, 1);
        assert_eq!(timer.task, "test");
        
        timer.task = "test2".to_string();
        storage.update_timer(timer).expect("err");
        let timer = storage.get_timer_by_id(id).unwrap();
        assert_eq!(timer.task, "test2");
    }

    #[test]
    fn test_select_by_status() {
        let mut storage = Storage::from_memory().expect("err");
        storage.insert_timer(&SQLTimerRow{
            id: 0,
            task: "test1".to_string(),
            start: str_to_time("2024-01-01 00:00:00".to_string()),
            end: str_to_time("2024-01-01 00:00:00".to_string()),
            idle: 0,
            status: 1
        }).expect("Problem");
        storage.insert_timer(&SQLTimerRow{
            id: 0,
            task: "test2".to_string(),
            start: str_to_time("2024-01-01 00:00:00".to_string()),
            end: str_to_time("2024-01-01 00:00:00".to_string()),
            idle: 0,
            status: 1
        }).expect("Problem");
        storage.insert_timer(&SQLTimerRow{
            id: 0,
            task: "test3".to_string(),
            start: str_to_time("2024-01-01 00:00:00".to_string()),
            end: str_to_time("2024-01-01 00:00:00".to_string()),
            idle: 0,
            status: 2
        }).expect("Problem");
        let items = storage.get_timers_by_status(1, -1).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_select_by_date() {
        let mut storage = Storage::from_memory().expect("err");
        storage.insert_timer(&SQLTimerRow{
            id: 0,
            task: "test1".to_string(),
            start: str_to_time("2024-01-01 00:00:00".to_string()),
            end: str_to_time("2024-01-01 00:00:00".to_string()),
            idle: 0,
            status: 1
        }).expect("Problem");
        storage.insert_timer(&SQLTimerRow{
            id: 0,
            task: "test2".to_string(),
            start: str_to_time("2024-01-02 00:00:00".to_string()),
            end: str_to_time("2024-01-02 00:00:00".to_string()),
            idle: 0,
            status: 1
        }).expect("Problem");
        storage.insert_timer(&SQLTimerRow{
            id: 0,
            task: "test3".to_string(),
            start: str_to_time("2024-01-03 00:00:00".to_string()),
            end: str_to_time("2024-01-03 00:00:00".to_string()),
            idle: 0,
            status: 1
        }).expect("Problem");

        let items = storage.get_timers_by_date(
            -1,
            Some(str_to_time("2024-01-02 00:00:00".to_string())),
            None
        ).unwrap();
        assert_eq!(items.len(), 2);

        let items = storage.get_timers_by_date(
            -1,
            None,
            Some(str_to_time("2024-01-01 00:00:00".to_string())),
        ).unwrap();
        assert_eq!(items.len(), 2);
    }

}
