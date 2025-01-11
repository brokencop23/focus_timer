use clap::{Parser, Subcommand};
use focus_timer::{Storage, TimerCollection};
use std::path::PathBuf;
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
    Start { 
        #[arg(short, long)]
        task: String
    },
    Info,
    Pause,
    Stop,
    Drop,
    Export {
        #[arg(long, short)]
        path: String,

        #[arg(long)]
        date_from: Option<String>,

        #[arg(long)]
        date_to: Option<String>,

        #[arg(long, short)]
        n: Option<i32>
    },
    List {
        #[arg(long)]
        date_from: Option<String>,

        #[arg(long)]
        date_to: Option<String>,

        #[arg(long, short)]
        n: Option<i32>
    },
    Stat {
        #[arg(long)]
        date_from: Option<String>,

        #[arg(long)]
        date_to: Option<String>,

        #[arg(long, short)]
        n: Option<i32>
    },
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
    let mut storage = Storage::build(db_path.into()).expect("DB not created");
    let mut collection = TimerCollection::new();
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Info) => {
            println!("Using database at: {}", storage.path().display())
        },
        Some(Commands::Start { task }) => {
            match storage.start_timer(task) {
                Ok(()) => println!("Task started"),
                Err(e) => panic!("{e}")
            };
        },
        Some(Commands::Drop) => {
            storage.drop().expect("Can't remove");
        }
        Some(Commands::Pause) => {
            match storage.pause_timer() {
                Ok(()) => println!("Task is paused"),
                Err(e) => panic!("{e}")
            };
        },
        Some(Commands::Stop) => {
            match storage.stop_timer() {
                Ok(()) => println!("Task is stopped"),
                Err(e) => panic!("{e}")
            };
        },
        Some(Commands::Export { path, date_from, date_to, n }) => {
            match storage.load_timers(
                &mut collection,
                n.unwrap_or(-1),
                date_from.clone(),
                date_to.clone()
            ) {
                Ok(()) => {},
                Err(e) => panic!("{e}")
            };
            match collection.export(path) {
                Ok(()) => println!("Saved"),
                Err(e) => panic!("{e}")
            }
        },
        Some(Commands::Stat { date_from, date_to, n }) => {
            match storage.load_timers(
                &mut collection,
                n.unwrap_or(-1),
                date_from.clone(),
                date_to.clone()
            ) {
                Ok(()) => {},
                Err(e) => panic!("{e}")
            };
            collection.print_stat();
        },
        Some(Commands::List { date_from, date_to, n }) => {
            match storage.load_timers(
                &mut collection,
                n.unwrap_or(-1),
                date_from.clone(),
                date_to.clone()
            ) {
                Ok(()) => {},
                Err(e) => panic!("{e}")
            };
            collection.print_stat();
            collection.print_items();
        }
        None => storage.display()
    }
}
