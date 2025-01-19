use clap::{Parser, Subcommand};
use focus_timer;
use focus_timer::Storage;
use std::path::PathBuf;
use std::fs;
use dirs;


fn get_default_db_path() -> PathBuf {
    if let Some(data_dir) = dirs::data_local_dir() {
        let app_dir = data_dir.join("focus_timer");
        return app_dir.join("database.db");
    }
    PathBuf::from("database.db")
}


#[derive(Subcommand)]
enum Commands {
    Info,
    New { 
        #[arg(short, long)]
        task: String
    },
    Start {
        #[arg(short, long)]
        id: i64
    },
    Stop {
        #[arg(short, long)]
        id: i64
    },
    Complete {
        #[arg(short, long)]
        id: i64
    },
    Delete {
        #[arg(short, long)]
        id: i64
    },
    Last { 
        #[arg(short)]
        n: u64
    },
    Flush,
    List {
        #[arg(long)]
        date_from: Option<String>,

        #[arg(long)]
        date_to: Option<String>,

        #[arg(long, short)]
        n: Option<i32>
    },
    Export {
        #[arg(long)]
        date_from: Option<String>,

        #[arg(long)]
        date_to: Option<String>,

        #[arg(short, long)]
        path: String
    },
    Stat {
        #[arg(long)]
        date_from: Option<String>,

        #[arg(long)]
        date_to: Option<String>,
    }
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>
}


fn main() {
    let db_path = std::env::var("APP_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| get_default_db_path());
    let storage = Storage::from_path(db_path.clone()).expect("DB not created");
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Info) => {
            println!("Using database at: {}", db_path.display())
        },
        Some(Commands::New { task }) => {
            match focus_timer::new_timer(&storage, task.to_string()) {
                Ok(id) => println!("Created timer {}", id),
                Err(e) => panic!("{e}")
            };
        },
        Some(Commands::Start { id }) => {
            match focus_timer::start_timer(&storage, *id) {
                Ok(()) => println!("Task started"),
                Err(e) => panic!("{e}")
            };
        },
        Some(Commands::Stop { id }) => {
            match focus_timer::stop_timer(&storage, *id) {
                Ok(()) => println!("Task is paused"),
                Err(e) => panic!("{e}")
            };
        },
        Some(Commands::Complete { id }) => {
            match focus_timer::complete_timer(&storage, *id) {
                Ok(()) => println!("Task is completed"),
                Err(e) => panic!("{e}")
            };
        },
        Some(Commands::Delete { id }) => {
            match focus_timer::delete_timer(&storage, *id) {
                Ok(()) => println!("Task is completed"),
                Err(e) => panic!("{e}")
            };
        },
        Some(Commands::Flush) => {
            match fs::remove_file(db_path) {
                Err(e) => panic!("{e}"),
                _ => println!("Database was deleted")
            }
        },
        Some(Commands::List { date_from, date_to, n }) => {
            match focus_timer::show_list(
                &storage,
                n.unwrap_or(-1),
                date_from.clone(),
                date_to.clone()
            ) {
                Ok(()) => {},
                Err(e) => panic!("{e}")
            };
        },
        Some(Commands::Export { date_from, date_to, path }) => {
            match focus_timer::export(
                &storage,
                path.clone(),
                date_from.clone(),
                date_to.clone()
            ) {
                Ok(()) => {},
                Err(e) => panic!("{e}")
            };
        },
        Some(Commands::Stat { date_from, date_to }) => {
            match focus_timer::show_stat(
                &storage,
                date_from.clone(),
                date_to.clone()
            ) {
                Ok(()) => {},
                Err(e) => panic!("{e}")
            };
        },
        Some(Commands::Last { n }) => {
            match focus_timer::show_last_n(&storage, *n) {
                Ok(()) => {},
                Err(e) => panic!("{e}")
            }
        },
        None => {
            match focus_timer::current_info(&storage) {
                Ok(()) => {},
                Err(e) => panic!("{e}")
            }
        }
    }
}
