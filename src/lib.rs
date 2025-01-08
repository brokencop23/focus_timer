use rusqlite;
use rusqlite::{Connection, Result};
use chrono::{DateTime, Utc};
use std::error::Error;
use std::fmt;

pub enum TimerError {
    AlreadyStopped
}

#[derive(Debug)]
pub struct Timer {
    task: String,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

impl Timer {

    pub fn new(task: &str) -> Self {
        let start: DateTime<Utc> = Utc::now();
        let timer = Timer {
            task: task.to_string(),
            start,
            end: start,
        };
        timer
    }

    pub fn stop(&mut self) -> Result<DateTime<Utc>, TimerError> {
        if self.end != self.start {
            return Err(TimerError::AlreadyStopped);
        }
        self.end = Utc::now();
        Ok(self.end)
    }

}

#[derive(Debug)]
pub enum StorageError {
    OpenTimerExists,
    TimerDoesNotExists
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StorageError::OpenTimerExists => write!(f, "Open timer already exists"),
            StorageError::TimerDoesNotExists => write!(f, "Timer does not exist"),
        }
    }
}

impl Error for StorageError {}


#[derive(Debug)]
pub struct Storage {
    conn: Connection
}

impl Storage {
    
    pub fn new(path: &str) -> Result<Storage, Box<dyn Error>> {
        let conn = Connection::open(&path)?;

        conn.execute("CREATE OR REPLACE TABLE timers (
            start INTEGER PRIMARY KEY,
            task STRING,
            end INTEGER
        ", [])?;

        Ok(Storage {
            conn: Connection::open(&path)?
        })
    }

    pub fn is_open_timer(&self) -> Result<bool, Box<dyn Error>> { 
        Ok(self.conn.query_row(
            "SELECT count(0) FROM timers WHERE start = end",
            [],
            |row| row.get::<_, i64>(0) 
        )? > 0)
    }

    pub fn is_timer_exist(&self, start: i64) -> Result<bool, Box<dyn Error>> {
        Ok(self.conn.query_row(
            "SELECT count(0) FROM timers WHERE start=?1",
            rusqlite::params![start],
            |row| row.get::<_, i64>(0)
        )? > 0)
    }

    pub fn add_timer(&self, timer: &Timer) -> Result<i64, Box<dyn Error>> {
        if self.is_open_timer()? {
            return Err(Box::new(StorageError::OpenTimerExists));
        }
        self.conn.execute(
            "INSERT INTO timers (start, task, end) VALUES (?1, ?2, ?3)",
            rusqlite::params![
                timer.start.timestamp(),
                timer.task,
                timer.end.timestamp()
            ]
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_timer(&self, timer: &Timer) -> Result<(), Box<dyn Error>> {
        if self.is_timer_exist(timer.start.timestamp())? == false {
            return Err(Box::new(StorageError::TimerDoesNotExists));
        }
        self.conn.execute(
            "UPDATE timers SET task='?1', end='?2' WHERE start=?3",
            rusqlite::params![
                timer.task,
                timer.end.timestamp(),
                timer.start.timestamp()
            ]
        )?;
        Ok(())
    }

    pub fn get_active_timer(&self) -> Result<Option<Timer>, Box<dyn Error>> {
        if self.is_open_timer()? == false {
            return Ok(None);
        }
        let result = self.conn.query_row(
            "SELECT start, task, end FROM timers WHERE start = end",
            [],
            |row| {
                Ok(Timer {
                    start: DateTime::from_timestamp(row.get(0)?, 0).unwrap(),
                    task: row.get(1)?,
                    end: DateTime::from_timestamp(row.get(2)?, 0).unwrap()
                })
            }
        );
        match result {
            Ok(timer) => Ok(Some(timer)),
            Err(e) => Err(Box::new(e))
        }
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::thread::sleep;
    use std::fs;

    fn remove_db(p: &str) {
        if fs::exists(p).unwrap() {
            fs::remove_file(p).unwrap();
        }
    }

    #[test]
    fn test_timer_new() {
        let timer = Timer::new(&String::from("Test"));
        assert!(timer.start.timestamp() > 0);
        assert!(timer.end.timestamp() > 0);
        assert!(timer.start.eq(&timer.end));
    }

    #[test]
    fn test_timer_end() {
        let mut timer = Timer::new(&"Test");
        sleep(Duration::from_secs(1));
        let _ = timer.stop();
        assert!(timer.end.timestamp() > timer.start.timestamp());
        match timer.stop() {
            Err(TimerError::AlreadyStopped) => (),
            _ => panic!("Expected AlreadyStopped")
        }
    }
    
    #[test]
    fn test_get_active_empty() {
        remove_db("test.db");
        let storage = Storage::new("test.db").expect("Error");
        let timer = storage.get_active_timer();
        match timer {
            Ok(None) => (),
            _ => panic!("Expected None")
        }
    }

}
