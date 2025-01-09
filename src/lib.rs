use rusqlite;
use rusqlite::{Connection, Result};
use chrono::{DateTime, Utc};
use std::error::Error;
use std::fmt;

const SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(i32)]
enum TimerStatus {
    RUN = 1,
    PAUSED = 2,
    CLOSED = 3
}

#[derive(Debug, Clone)]
pub struct Timer {
    task: String,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    idle: i64,
    status: TimerStatus
}

#[derive(Debug)]
pub enum StorageError {
    CurrentTimerNotClosed,
    TimerDoesNotExists,
    SchemaVersionError,
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
    current_timer: Option<Timer>
}

impl Storage {

    fn get_version(conn: &Connection) -> i32 {
        match conn.query_row(
            "SELECT version FROM schema_version",
            [],
            | row | { Ok(row.get(0)?) }
        ) {
            Ok(ver) => ver,
            Err(e) => panic!("Database error: {e}")
        }
    }

    pub fn new(path: &str) -> Result<Storage, Box<dyn Error>> {
        let conn = Connection::open(&path)?;

        conn.execute("CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER
        )", [])?;
        
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            rusqlite::params![SCHEMA_VERSION]
        )?;

        conn.execute("CREATE TABLE IF NOT EXISTS timers (
            start INTEGER PRIMARY KEY,
            task STRING,
            end INTEGER,
            idle INTEGER,
            status INTEGER
        )", [])?;

        if Storage::get_version(&conn) != SCHEMA_VERSION {
            return Err(Box::new(StorageError::SchemaVersionError));
        }

        Ok(Storage {
            conn, 
            current_timer: None
        })
    }

    pub fn build(path: &str) -> Result<Storage, Box<dyn Error>> {
        let mut storage = Storage::new(path)?;
        storage.current_timer = storage.get_active_timer();
        Ok(storage)
    }

    pub fn get_active_timer(&self) -> Option<Timer> {
        match self.conn.query_row(
            "SELECT task, start, end, idle, status FROM timers WHERE status = ?1",
            rusqlite::params![
                TimerStatus::CLOSED as i32
            ],
            |row| {
                Ok(Timer {
                    task: row.get(0)?,
                    start: DateTime::from_timestamp(row.get(1)?, 0).unwrap(),
                    end: DateTime::from_timestamp(row.get(2)?, 0).unwrap(),
                    idle: row.get(3)?,
                    status: match row.get(4)?{
                        1 => TimerStatus::RUN,
                        2 => TimerStatus::PAUSED,
                        3 => TimerStatus::CLOSED,
                        _ => panic!("not supported value")
                    },
                })
            }
        ) {
            Ok(timer) => Some(timer),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => panic!("Database error: {e}")
        }
    }

    pub fn start_timer(&mut self, task: &str) -> Result<(), Box<dyn Error>> {
        match &self.current_timer {
            Some(_) => {
                Err(Box::new(StorageError::CurrentTimerNotClosed))
            },
            None => {
                let now = Utc::now();

                let timer = Timer {
                    start: now,
                    end: now,
                    task: task.to_string(),
                    status: TimerStatus::RUN,
                    idle: 0
                };

                self.conn.execute(
                    "INSERT INTO timers (start, end, task, idle, status)
                        VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        timer.start.timestamp(),
                        timer.end.timestamp(),
                        timer.task,
                        timer.idle,
                        timer.status as i32
                    ]
                )?;

                self.current_timer = Some(timer);

                Ok(())
            }
        }
    }

    pub fn pause_timer(&mut self) -> Result<(), Box<dyn Error>> {
        match &mut self.current_timer {
            None => {
                Err(Box::new(StorageError::TimerDoesNotExists))
            },
            Some(timer) => {
                let now = Utc::now(); 
                match timer.status {
                    TimerStatus::PAUSED => {
                        timer.idle += now.timestamp() - timer.end.timestamp();
                        timer.status = TimerStatus::RUN;
                    },
                    TimerStatus::RUN => {
                        timer.end = now;
                        timer.status = TimerStatus::PAUSED;
                    },
                    TimerStatus::CLOSED => {
                        return Err(Box::new(StorageError::TimerHasBeenClosed))
                    }
                }
                self.conn.execute(
                    "UPDATE timers SET end=?1, status=?2, idle=?3 WHERE start=?4",
                    rusqlite::params![
                        timer.end.timestamp(),
                        timer.status as i32,
                        timer.idle,
                        timer.start.timestamp()
                    ]
                )?;
                Ok(())
            }
        }
    }

    pub fn stop_timer(&mut self) -> Result<(), Box<dyn Error>> {
        match &self.current_timer {
            None => {
                Err(Box::new(StorageError::TimerDoesNotExists))
            },
            Some(timer) => {
                let now = Utc::now();
                self.conn.execute(
                    "UPDATE timers SET end = ?1 WHERE start = ?",
                    rusqlite::params![
                        now.timestamp(),
                        timer.start.timestamp()
                    ]
                )?;

                self.current_timer = None;
                Ok(())
            }
        }
    }


}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn remove_db(p: &str) {
        if fs::exists(p).unwrap() {
            fs::remove_file(p).unwrap();
        }
    }
    
    #[test]
    fn test_create_storage() {
        let db = "1.db";
        let storage = Storage::new(db);
        match storage {
            Ok(_) => (),
            Err(e) => panic!("Error {e}")
        }
        remove_db(db);
    }

    #[test]
    fn test_timer_cycle() {
        let db = "2.db";
        let mut storage = Storage::new(db).expect("Some error");

        storage.start_timer("test task").expect("Failed");
        let timer = storage.current_timer.clone().unwrap();
        assert_eq!(timer.task, "test task");
        assert_eq!(timer.status, TimerStatus::RUN);

        storage.pause_timer().expect("Failed");
        let timer = storage.current_timer.clone().unwrap();
        assert_eq!(timer.status, TimerStatus::PAUSED);

        std::thread::sleep(std::time::Duration::from_secs(2));

        storage.pause_timer().expect("Failed");
        let timer = storage.current_timer.clone().unwrap();
        assert_eq!(timer.status, TimerStatus::RUN);
        assert!(timer.idle >= 2);

        storage.stop_timer().expect("Failed");
        assert!(storage.current_timer.is_none());
        remove_db(db);
    }

    #[test]
    fn test_storage_start_timer() {
        let db = "3.db";
        let mut storage = Storage::new(db).expect("Some error");
        storage.start_timer("test task").expect("Failed");
        let timer = storage.current_timer.clone().unwrap();
        assert_eq!(timer.task, "test task");
        assert_eq!(timer.status, TimerStatus::RUN);
        match storage.start_timer("second task") {
            Ok(_) => panic!("Second timer should not have started"),
            Err(e) => {
                assert!(matches!(
                    e.downcast_ref::<StorageError>(),
                    Some(StorageError::CurrentTimerNotClosed)
                ));
            }
        }
        remove_db(db);
    }

}
