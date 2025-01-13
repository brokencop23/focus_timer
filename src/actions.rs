use crate::{TimerCollection, Timer};
use crate::{Storage, SQLTimerRow};



    pub fn display(&self) {
        match &self.current_timer {
            Some(t) => t.print(),
            None => println!("There is no active timer")
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
    pub fn get_active_timer(&self) -> Option<SQLTimerRow> {
        match self.conn.query_row(
            rusqlite::params![
                TimerStatus::CLOSED as i32
            ],
            | row | {
                Ok(SQLTimerRow {
                    task: row.get(0)?,
                    start: row.get(1)?,
                    end: row.get(2)?,
                    idle: row.get(3)?,
                    status: row.get(4)?
                })
            }
        ) {
            Ok(timer) => Some(timer),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => panic!("Database error: {e}")
        }
    }
    pub fn pause_timer(&mut self) -> Result<(), Box<dyn Error>> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
