mod timer;
mod storage;

use std::error::Error;
use std::fmt;
pub use timer::{TimerStatus, Timer, TimerCollection, TimerError};
pub use storage::{Storage, SQLTimerRow, StorageError};


#[derive(Debug)]
pub enum LogicError {
    ActiveTimerExists
}

impl fmt::Display for LogicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogicError::ActiveTimerExists => write!(f, "Active timer exist") 
        }
    }
}
impl Error for LogicError {}

pub fn new_timer(
    storage: &Storage,
    task: String
) -> Result<i64, StorageError> {
    let timer = Timer::from(task);
    let id = storage.insert_timer(&timer.to_sqlite_row())?;
    Ok(id)
}

pub fn start_timer(storage: &Storage, id: i64) -> Result<(), Box<dyn Error>> {
    if storage.count_timers_by_status(TimerStatus::RUN as u32)? > 0 {
        return Err(Box::new(LogicError::ActiveTimerExists));
    }
    let mut timer = Timer::from(storage.get_timer_by_id(id)?);
    timer.start()?;
    storage.update_timer(&timer.to_sqlite_row())?;
    Ok(())
}

pub fn stop_timer(storage: &Storage, id: i64) -> Result<(), Box<dyn Error>> {
    let mut timer = Timer::from(storage.get_timer_by_id(id)?);
    timer.stop()?;
    storage.update_timer(&timer.to_sqlite_row())?;
    Ok(())
}

pub fn complete_timer(storage: &Storage, id: i64) -> Result<(), Box<dyn Error>> {
    let mut timer = Timer::from(storage.get_timer_by_id(id)?);
    timer.complete()?;
    storage.update_timer(&timer.to_sqlite_row())?;
    Ok(())
}

pub fn current_info(storage: &Storage) -> Result<(), Box<dyn Error>> {
    let collection = TimerCollection::from(
        storage.get_timers_by_status(TimerStatus::RUN as u32, -1)?
    );
    println!("=== Active task ===");
    if collection.size() == 0 {
        println!("No active task");
    } else {
        collection.print_items();
    }
    Ok(())
}

pub fn show_list(
    storage: &Storage,
    limit: i32,
    date_from: Option<String>,
    date_to: Option<String>
) -> Result<(), Box<dyn Error>> {
    let collection = TimerCollection::from(
        storage.get_timers_by_date(limit, date_from, date_to)?
    );
    collection.print_items();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow() {
        let storage = Storage::from_memory().expect("err");
        let id = new_timer(&storage, String::from("test")).expect("err");
        assert_eq!(id, 1);
        let timer = Timer::from(storage.get_timer_by_id(id).expect("err"));
        assert_eq!(timer.status, TimerStatus::NEW);
    }
}
