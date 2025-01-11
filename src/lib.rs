use rusqlite;
use rusqlite::{Connection, Result};
use chrono::{DateTime, Utc};
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

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

impl fmt::Display for TimerStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TimerStatus::RUN => write!(f, "Running"),
            TimerStatus::PAUSED => write!(f, "Paused"),
            TimerStatus::CLOSED => write!(f, "Finished")
        }
    }
}

impl Error for StorageError {}


impl Timer {
    fn time_on(&self) -> i64 {
        let time = match self.status {
            TimerStatus::RUN => {
                Utc::now().timestamp() - self.start.timestamp()
            },
            _ => {
                self.end.timestamp() - self.start.timestamp()
            }
        };
        time - self.idle
    }
    
    fn print_time_on(time: i64) {
        match time {
            t if t <= 60 => println!("{} sec", t),
            t if t <= 3600 => {
                println!(
                    "{} min {} sec",
                    t / 60,
                    t % 60
                );
            },
            t if t <= 86400 => {
                println!(
                    "{} hours {} min {} sec",
                    t / 3600,
                    (t % 3600) / 60,
                    t % 60
                )
            },
            t if t <= 86400 => {
                println!(
                    "{} hours {} min {} sec",
                    t / 3600,
                    (t % 3600) / 60,
                    t % 60
                )
            },
            t if t <= 86400 => {
                println!(
                    "{} hours {} min {} sec",
                    t / 3600,
                    (t % 3600) / 60,
                    t % 60
                )
            },
            t => {
                println!(
                    "{} days {} hr {} min {} sec",
                    t / 86400,
                    (t % 86400) / 3600,
                    (t % 3600) / 60,
                    t % 60
                );
            }
        }
    }

    pub fn print(&self) {
        println!("\n=========================");
        println!("Current task: {}", self.task);
        println!("Started at: {}", self.start);
        println!("Status: {}", self.status);
        print!("Spent: ");
        Timer::print_time_on(self.time_on());
        println!("\n=========================");
    }
}

pub struct TimerCollection {
    items: Vec<Timer>
}

impl TimerCollection {
    
    pub fn new() -> TimerCollection {
        TimerCollection { items: Vec::new() }
    }

    pub fn print_items(&self) {
        self.items.iter().for_each(| t | t.print());
    }

    pub fn print_stat(&self) {
        let mut n = 0;
        let mut time_on = 0;
        self.items.iter().for_each(| t | {
            n += 1;
            time_on += t.time_on()
        });
        println!("==>> TOTAL STAT <<==");
        println!("N tasks: {n}");
        print!("Total time: ");
        Timer::print_time_on(time_on);

        print!("Avg time: ");
        Timer::print_time_on(time_on / n as i64);
        
    }

    pub fn export(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let mut f = fs::File::create(path)?;
        let mut n = 1;
        writeln!(f, "n,start,end,status,time_on")?;
        for t in self.items.iter() {
            writeln!(
                f,
                "{},{},{},{},{}",
                n,
                t.start,
                t.end,
                t.status,
                t.time_on()
            )?;
            n += 1;
        };
        Ok(())
    }

}


#[derive(Debug)]
pub struct Storage {
    conn: Connection,
    path: PathBuf,
    current_timer: Option<Timer>
}

impl Storage {

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

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

    pub fn new(path: PathBuf) -> Result<Storage, Box<dyn Error>> {
        let conn = Connection::open(&path)?;

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
            start INTEGER PRIMARY KEY,
            task STRING,
            end INTEGER,
            idle INTEGER,
            status INTEGER
        )", [])?;


        Ok(Storage {
            conn,
            path,
            current_timer: None
        })
    }

    pub fn build(path: PathBuf) -> Result<Storage, Box<dyn Error>> {
        let mut storage = Storage::new(path)?;
        storage.current_timer = storage.get_active_timer();
        Ok(storage)
    }

    pub fn display(&self) {
        match &self.current_timer {
            Some(t) => t.print(),
            None => println!("There is no active timer")
        }
    }

    pub fn drop(&self) -> Result<(), Box<dyn Error>> {
        if fs::exists(&self.path).unwrap_or(false) == true {
            fs::remove_file(&self.path)?
        }
        Ok(())
    }

    pub fn get_active_timer(&self) -> Option<Timer> {
        match self.conn.query_row(
            "SELECT task, start, end, idle, status FROM timers WHERE status != ?1",
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
            Some(_t) => {
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
                let now = match timer.status {
                    TimerStatus::RUN => Utc::now().timestamp(),
                    _ => timer.end.timestamp()
                };
                self.conn.execute(
                    "UPDATE timers SET end=?1, status=?2 WHERE start=?3",
                    rusqlite::params![
                        now,
                        TimerStatus::CLOSED as i32,
                        timer.start.timestamp()
                    ]
                )?;
                self.current_timer = None;
                Ok(())
            }
        }
    }

    pub fn load_timers(
        &self,
        collection: &mut TimerCollection,
        n: i32,
        date_from: Option<String>,
        date_to: Option<String>
    ) -> Result<(), Box<dyn Error>> {
        let date_from = date_from.map(|d| {
            DateTime::parse_from_str(
                format!("{} 00:00:00 +0000", d).as_str(),
                "%Y-%m-%d %H:%M:%S %z"
            ).unwrap().timestamp()
        });
        let date_to = date_to.map(|d| {
            DateTime::parse_from_str(
                format!("{} 00:00:00 +0000", d).as_str(),
                "%Y-%m-%d %H:%M:%S %z"
            ).unwrap().timestamp()
        });
        let query = "
            SELECT task, start, end, idle, status
            FROM timers
            WHERE
                (?1 is NULL OR start >= ?1)
                AND (?2 is NULL OR start <= ?2)
            ORDER BY start DESC
            LIMIT COALESCE(?3, -1)
        ";
        let mut stmt = self.conn.prepare(query)?;
        let items = stmt.query_map(
            rusqlite::params![date_from, date_to, n],
            | row | {
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
        )?;
        collection.items.clear();
        collection.items.extend(items.filter_map(Result::ok));
        Ok(())
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
