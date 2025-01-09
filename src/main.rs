use clap::{Parser, Subcommand};
use focus_timer::{Timer, Storage};
use std::error::Error;

const DB_PATH: &str = "test.db";


#[derive(Subcommand)]
enum Commands {
    Start { 
        #[arg(short, long)]
        task: String
    },
    Pause,
    Stop,
    Export {
        #[arg(short, long)]
        date_from: String,

        #[arg(short, long)]
        date_to: String
    },
    Stat {
        #[arg(short, long)]
        date_from: String,

        #[arg(short, long)]
        date_to: String
    }
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>
}

fn start_timer(task: &str, storage: &Storage) {
    let timer = Timer::new(task);
    let _ = storage.add_timer(&timer).expect("Timer was not added");
}

fn main() {
    let storage = Storage::new(DB_PATH).expect("DB not created");
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Start { task }) => start_timer(task, &storage),
        Some(Commands::Pause) => todo!(),
        Some(Commands::Stop) => todo!(),
        Some(Commands::Export { date_from, date_to }) => todo!(),
        Some(Commands::Stat { date_from, date_to }) => todo!(),
        None => todo!()
    }
}
