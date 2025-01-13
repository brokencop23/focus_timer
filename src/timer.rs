use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;
use chrono::{DateTime, Utc};
use crate::SQLTimerRow;


#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(i32)]
pub enum TimerStatus {
    RUN = 1,
    PAUSED = 2,
    CLOSED = 3
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

impl TimerStatus {
    pub fn from_int(n: u32) -> TimerStatus {
        match n {
            1 => TimerStatus::RUN,
            2 => TimerStatus::PAUSED,
            3 => TimerStatus::CLOSED,
            _ => panic!("not supported value")
        }
    }
}

#[derive(Debug, Clone)]
pub struct Timer {
    id: u64,
    task: String,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    idle: i64,
    status: TimerStatus
}

impl Timer {

    pub fn from_sqlite_row(row: SQLTimerRow) -> Timer {
        let start = DateTime::from_timestamp(row.start as i64, 0).unwrap();
        let end = DateTime::from_timestamp(row.end as i64, 0).unwrap();
        let status = TimerStatus::from_int(row.status);
        Timer {
            start,
            end,
            id: row.id,
            task: row.task,
            idle: row.idle,
            status
        }
    }

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
