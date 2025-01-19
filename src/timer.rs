use std::fmt;
use std::fs;
use std::io::Write;
use serde;
use serde::Serialize;
use std::error::Error;
use chrono::{DateTime, Utc};
use crate::SQLTimerRow;


#[derive(Debug, PartialEq)]
pub enum TimerError {
    TimerHasFiniteState,
}

impl fmt::Display for TimerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TimerError::TimerHasFiniteState => write!(f, "This timer cannot be changed"),
        }
    }
}

impl Error for TimerError {}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(i32)]
pub enum TimerStatus {
    NEW = 0,
    RUN = 1,
    PAUSED = 2,
    COMPLETED = 3,
    DELETED = 9 
}

impl fmt::Display for TimerStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TimerStatus::NEW => write!(f, "New"),
            TimerStatus::RUN => write!(f, "Running"),
            TimerStatus::PAUSED => write!(f, "Paused"),
            TimerStatus::COMPLETED => write!(f, "Completed"),
            TimerStatus::DELETED => write!(f, "Deleted")
        }
    }
}

impl Serialize for TimerStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            serializer.serialize_str(&self.to_string())
        }
}

impl From<u32> for TimerStatus {
    fn from(n: u32) -> TimerStatus {
        match n {
            0 => TimerStatus::NEW,
            1 => TimerStatus::RUN,
            2 => TimerStatus::PAUSED,
            3 => TimerStatus::COMPLETED,
            9 => TimerStatus::DELETED,
            _ => panic!("not supported value")
        }
    }
}


#[derive(Debug, Clone, Serialize)]
pub struct Timer {
    pub id: i64,
    pub task: String,
    #[serde(serialize_with="serialize_datetime")] 
    pub start: DateTime<Utc>,
    #[serde(serialize_with="serialize_datetime")] 
    pub end: DateTime<Utc>,
    pub idle: i64,
    pub status: TimerStatus
}

fn serialize_datetime<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where 
    S: serde::Serializer {
        let formatted = dt.format("%Y-%m-%d %H:%M:%S").to_string();
        serializer.serialize_str(&formatted)
    }

impl From<SQLTimerRow> for Timer {
    fn from(row: SQLTimerRow) -> Self {
        let start = DateTime::from_timestamp(row.start as i64, 0).unwrap();
        let end = DateTime::from_timestamp(row.end as i64, 0).unwrap();
        let status = TimerStatus::from(row.status);
        Self::new(row.id, row.task, start, end, row.idle, status)
    }
}

impl From<String> for Timer {
    fn from(task: String) -> Self {
        let t = Utc::now();
        Self::new(0, task, t, t, 0, TimerStatus::NEW)
    }
}

impl Timer {

    pub fn new(
        id: i64,
        task: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        idle: i64,
        status: TimerStatus
    ) -> Self {
        Self { id, task, start, end, idle, status }
    }

    pub fn set_start(&mut self) -> Result<(), TimerError>  {
        match self.status {
            TimerStatus::DELETED | TimerStatus::COMPLETED => Err(TimerError::TimerHasFiniteState),
            TimerStatus::NEW => {
                let now = Utc::now();
                self.start = now;
                self.end = now;
                self.status = TimerStatus::RUN;
                Ok(()) 
            },
            TimerStatus::PAUSED => {
                let now = Utc::now(); 
                self.idle = now.timestamp() - self.end.timestamp();
                self.status = TimerStatus::RUN;
                Ok(())
            },
            _ => {
                self.status = TimerStatus::RUN;
                Ok(())
            }
        }
    }

    pub fn set_stop(&mut self) -> Result<(), TimerError> {
        if self.status == TimerStatus::DELETED 
           || self.status == TimerStatus::COMPLETED {
            return Err(TimerError::TimerHasFiniteState)
        }
        if self.status == TimerStatus::NEW { self.set_start()? };
        if self.status == TimerStatus::PAUSED { return Ok(()) };
        self.end = Utc::now();
        self.status = TimerStatus::PAUSED;
        Ok(())
    }

    pub fn set_complete(&mut self) -> Result<(), TimerError> {
        if self.status == TimerStatus::NEW { self.set_start()?; };
        if self.status == TimerStatus::RUN { self.set_stop()?; };
        self.status = TimerStatus::COMPLETED;
        Ok(())
    }

    pub fn set_delete(&mut self) -> Result<(), TimerError> {
        self.status = TimerStatus::DELETED;
        Ok(())
    }

    pub fn to_sqlite_row(&self) -> SQLTimerRow {
        SQLTimerRow {
            id: self.id,
            task: self.task.clone(),
            start: DateTime::<Utc>::timestamp(&self.start) as u64,
            end: DateTime::<Utc>::timestamp(&self.end) as u64,
            idle: self.idle,
            status: self.status as u32
        }
    }

    pub fn time_on(&self) -> i64 {
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
        println!("id: {}", self.id);
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

impl From<Vec<SQLTimerRow>> for TimerCollection {
    fn from(items: Vec<SQLTimerRow>) -> Self {
        Self {
            items: items.into_iter().map(Timer::from).collect()
        }
    }
}

impl TimerCollection {
    
    pub fn new() -> TimerCollection {
        TimerCollection { items: Vec::new() }
    }
    
    pub fn size(&self) -> usize {
        self.items().len()
    }

    pub fn items(&self) -> Vec<&Timer> {
        self.items
            .iter()
            .filter(|x| x.status != TimerStatus::DELETED)
            .collect()
    }

    pub fn print_items(&self) {
        self.items().iter().for_each(| t | t.print());
    }

    pub fn print_stat(&self) {
        let mut n = 0;
        let mut time_on = 0;
        let mut time_on_compl = 0;
        let mut n_compl = 0;
        self.items().iter().for_each(| t | {
            n += 1;
            if t.status == TimerStatus::COMPLETED {
                n_compl += 1;
                time_on_compl += t.time_on()
            }
            time_on += t.time_on()
        });
        println!("==>> TOTAL STAT <<==");
        println!("N tasks: {n}");
        println!("N completed: {n_compl}");
        if n > 0 {
            println!("% comletion: {:.1}%", n_compl / n * 100);
            print!("Total time: ");
            Timer::print_time_on(time_on);
            print!("Avg time: ");
            Timer::print_time_on(time_on / n as i64);
        }
        if n_compl > 0 {
            print!("Total time (Completed): ");
            Timer::print_time_on(time_on_compl);
            print!("Avg time (Completed): ");
            Timer::print_time_on(time_on_compl / n_compl as i64);
        }
    }

    pub fn export(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let mut f = fs::File::create(path)?;
        let mut n = 1;
        writeln!(f, "n,start,end,status,time_on")?;
        for t in self.items().iter() {
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


#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::thread::sleep;

    #[test]
    fn test_start() {
        let mut timer = Timer::from("test".to_string());
        assert_eq!(timer.status, TimerStatus::NEW);
        match timer.set_start() {
            Ok(_) => assert_eq!(timer.status, TimerStatus::RUN),
            Err(_) => assert!(false)
        }
    }
    
    #[test]
    fn test_convert() {
        let timer = Timer::from("test".to_string());
        let timer_conv = timer.to_sqlite_row();
        assert_eq!(timer_conv.status, TimerStatus::NEW as u32)
    }
    
    #[test]
    fn test_start_completed() {
        let mut timer = Timer::from("test".to_string());
        timer.status = TimerStatus::COMPLETED;
        match timer.set_start() {
            Err(TimerError::TimerHasFiniteState) => assert!(true),
            _ => assert!(false)
        }
    }

    #[test]
    fn test_full_flow() {
        let mut t = Timer::from("test".to_string());
        assert_eq!(t.status, TimerStatus::NEW);
        
        match t.set_start() {
            Ok(_) => assert_eq!(t.status, TimerStatus::RUN),
            Err(e) => assert!(false, "{e}")
        }

        match t.set_stop() {
            Ok(_) => assert_eq!(t.status, TimerStatus::PAUSED),
            Err(e) => assert!(false, "{e}")
        }

        sleep(Duration::from_secs(1));

        match t.set_start() {
            Ok(_) => {
                assert_eq!(t.status, TimerStatus::RUN);
                assert!(t.idle > 0);
            }
            Err(e) => assert!(false, "{e}")
        }

        match t.set_start() {
            Ok(_) => assert_eq!(t.status, TimerStatus::RUN),
            Err(e) => assert!(false, "{e}")
        }

        match t.set_complete() {
            Ok(_) => assert_eq!(t.status, TimerStatus::COMPLETED),
            Err(e) => assert!(false, "{e}")
        }

        match t.set_start() {
            Err(TimerError::TimerHasFiniteState) => assert!(true),
            Ok(_) => assert!(false)
        }

    }
}
