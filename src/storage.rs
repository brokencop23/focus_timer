use rusqlite;
use rusqlite::{Connection, Row};
use std::fmt;
use std::error::Error;
use std::path::PathBuf;
use chrono::{NaiveDateTime, DateTime, Utc};


const SCHEMA_VERSION: i32 = 1;

#[derive(Debug)]
pub struct SQLTimerRow {
    pub id: i64,
    pub task: String,
    pub start: u64,
    pub end: u64,
    pub idle: i64,
    pub status: u32
}

impl SQLTimerRow {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            task: row.get("task")?,
            start: row.get("start")?,
            end: row.get("end")?,
            idle: row.get("idle")?,
            status: row.get("status")?
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum StorageError {
    DatabaseError(rusqlite::Error),
    SchemaVersionError,
    TimerDoesNotExists,
    ConnectionNotFound,
    WrongDatetimeFormat
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StorageError::TimerDoesNotExists => write!(f, "Timer does not exist"),
            StorageError::SchemaVersionError => write!(f, "Version of db is no correct"),
            StorageError::ConnectionNotFound => write!(f, "Connection to storage is not found"),
            StorageError::DatabaseError(e) => write!(f, "DatabaseError: {e}"),
            StorageError::WrongDatetimeFormat => write!(f, "Wrong date time format")
        }
    }
}

impl Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(error: rusqlite::Error) -> StorageError {
        StorageError::DatabaseError(error)
    }
}


#[derive(Debug)]
pub struct Storage {
    conn: Connection
}

impl Storage {

    fn get_version(&self) -> Result<Option<i32>, StorageError> {
        match self.conn.query_row(
            "SELECT value_int FROM db_params WHERE param == 'version'",
            [],
            | row | row.get(0) 
        ) {
            Ok(ver) => Ok(ver),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::DatabaseError(e))
        }
    }

    pub fn from_memory() -> Result<Self, StorageError> {
        Self::new(None)
    }

    pub fn from_path(path: PathBuf) -> Result<Self, StorageError> {
        Self::new(Some(path))
    }

    pub fn str_to_time(time_s: String) -> Result<u64, StorageError>{
        let time_s = time_s.trim();
        if time_s.is_empty() {
            return Err(StorageError::WrongDatetimeFormat);
        }
        let formats = [
            "%Y-%m-%d",
            "%Y-%m-%d %H:%M",
            "%Y-%m-%d %H:%M:%S"
        ];
        let normalized_time = if !time_s.contains(':') {
            format!("{} 00:00:00", time_s)
        } else if time_s.matches(':').count() == 1 {
            format!("{}:00", time_s)
        } else {
            time_s.to_string()
        };
        for format in formats {
            if let Ok(dt) = NaiveDateTime::parse_from_str(&normalized_time, format) {
                return Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc).timestamp() as u64);
            }
        }
        Err(StorageError::WrongDatetimeFormat)
    }

    pub fn new(path: Option<PathBuf>) -> Result<Self, StorageError> {
        let storage = Storage {
            conn: if let Some(path) = path {
                Connection::open(path)?
            } else {
                Connection::open_in_memory()?
            }
        };

        storage.conn.execute("CREATE TABLE IF NOT EXISTS db_params (
            param STRING,
            value_int INTEGER,
            value_str STRING,
            value_float FLOAT
        )", [])?;
       
        storage.conn.execute("CREATE TABLE IF NOT EXISTS timers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            start INTEGER,
            task STRING,
            end INTEGER,
            idle INTEGER,
            status INTEGER
        )", [])?;

        match storage.get_version()? {
            Some(ver) => {
                if ver != SCHEMA_VERSION {
                    return Err(StorageError::SchemaVersionError);
                }
            },
            None => {
                storage.conn.execute(
                    "INSERT INTO db_params (param, value_int) VALUES (?1, ?2)",
                    rusqlite::params!["version", SCHEMA_VERSION]
                )?;
            }
        }

        Ok(storage)
    }

    pub fn is_timer_exist(&self, id: i64) -> Result<bool, StorageError> {
        match self.conn.query_row(
            "SELECT count(0) AS n FROM timers WHERE id = ?1",
            rusqlite::params![id],
            | row | row.get::<_, u32>(0)
        ) {
            Ok(_) => Ok(true),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(StorageError::DatabaseError(e))
        }
    }

    pub fn insert_timer(&self, timer: &SQLTimerRow) -> Result<i64, StorageError> {
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

    pub fn update_timer(&self, timer: &SQLTimerRow) -> Result<(), StorageError> {
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

    pub fn get_timer_by_id(&self, id: i64) -> Result<SQLTimerRow, StorageError> {
        let q = "
            SELECT id, task, start, end, idle, status
            FROM timers
            WHERE id = ?1
        ";
        match self.conn.query_row(q, rusqlite::params![id], | r | SQLTimerRow::from_row(r)) {
            Ok(t) => Ok(t),
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(StorageError::TimerDoesNotExists),
            Err(e) => Err(StorageError::DatabaseError(e))
        }
    }

    pub fn count_timers_by_status(&self, status: u32) -> Result<u64, StorageError> {
        match self.conn.query_row("
            SELECT count() n
            FROM timers
            WHERE status = ?1",
            rusqlite::params![status],
            | r | r.get(0)
        ) {
            Ok(n) => Ok(n),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(StorageError::DatabaseError(e))
        }
    }

    pub fn get_timers_by_status(
        &self,
        status: u32,
        limit: i32
    ) -> Result<Vec<SQLTimerRow>, StorageError> {
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
            | row | SQLTimerRow::from_row(row)
        )?;
        Ok(items.filter_map(Result::ok).collect())
    }

    pub fn get_timers_by_date(
        &self,
        limit: i32,
        date_from: Option<String>,
        date_to: Option<String>
    ) -> Result<Vec<SQLTimerRow>, StorageError> {
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
        let from_timestamp = match date_from {
            Some(t) => Some(Self::str_to_time(t)?),
            None => None
        };
        let to_timestamp = match date_to {
            Some(t) => Some(Self::str_to_time(t)?),
            None => None
        };
        let items = stmt.query_map(
            rusqlite::params![from_timestamp, to_timestamp, limit],
            | row | SQLTimerRow::from_row(row)
        )?;
        Ok(items.filter_map(Result::ok).collect())
    }

}



#[cfg(test)]
mod tests {
    use super::*;

    fn setup_storage() -> Storage {
        let storage = Storage::from_memory().expect("err");
        let items = vec![
            SQLTimerRow {
                id: 0,
                task: "test1".to_string(),
                start: Storage::str_to_time("2024-01-01 00:00:00".to_string()).expect("err"),
                end: Storage::str_to_time("2024-01-01 00:00:00".to_string()).expect("err"),
                idle: 0,
                status: 1
            },
            SQLTimerRow {
                id: 0,
                task: "test2".to_string(),
                start: Storage::str_to_time("2024-01-02 00:00:00".to_string()).expect("err"),
                end: Storage::str_to_time("2024-01-02 00:00:00".to_string()).expect("err"),
                idle: 0,
                status: 1
            },
            SQLTimerRow {
                id: 0,
                task: "test3".to_string(),
                start: Storage::str_to_time("2024-01-03 00:00:00".to_string()).expect("err"),
                end: Storage::str_to_time("2024-01-03 00:00:00".to_string()).expect("err"),
                idle: 0,
                status: 1
            },
            SQLTimerRow {
                id: 0,
                task: "test4".to_string(),
                start: Storage::str_to_time("2024-01-04 00:00:00".to_string()).expect("err"),
                end: Storage::str_to_time("2024-01-04 00:00:00".to_string()).expect("err"),
                idle: 0,
                status: 2
            }
        ];
        for item in items {
            storage.insert_timer(&item).expect("Problem");
        }
        storage
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
            start: Storage::str_to_time("2024-01-01 00:00:00".to_string()).expect("err"),
            end: Storage::str_to_time("2024-01-01 00:00:00".to_string()).expect("err"),
            idle: 0,
            status: 1
        };
        let storage = Storage::from_memory().expect("err");
        let id = storage.insert_timer(&row).expect("Problem");
        let mut timer = storage.get_timer_by_id(id).unwrap();
        assert_eq!(id, 1);
        assert_eq!(timer.task, "test");
        
        timer.task = "test2".to_string();
        storage.update_timer(&timer).expect("err");
        let timer = storage.get_timer_by_id(id).unwrap();
        assert_eq!(timer.task, "test2");
    }

    #[test]
    fn test_select_by_status() {
        let storage = setup_storage();
        let items = storage.get_timers_by_status(1, -1).unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_select_by_date() {
        let storage = setup_storage();
        let items = storage.get_timers_by_date(
            -1,
            Some("2024-01-02".to_string()),
            None
        ).unwrap();
        assert_eq!(items.len(), 3);

        let items = storage.get_timers_by_date(
            -1,
            None,
            Some("2024-01-03".to_string()),
        ).unwrap();
        assert_eq!(items.len(), 2);

        let items = storage.get_timers_by_date(
            -1,
            Some("2024-01-02".to_string()),
            Some("2024-01-03".to_string())
        ).unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_update_status() {
        let storage = setup_storage();
        let mut item = storage.get_timer_by_id(1).unwrap();
        item.status = 3;
        storage.update_timer(&item).expect("err");
        
        let upd_item = storage.get_timer_by_id(item.id).unwrap();
        assert_eq!(upd_item.status, 3);
    }

    #[test]
    fn test_err_exist() {
        let storage = setup_storage();
        let item = storage.get_timer_by_id(300);
        assert!(item.is_err());
        match item {
            Err(StorageError::TimerDoesNotExists) => assert!(true),
            _ => assert!(false)
        }
    }

}
