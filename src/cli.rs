use crate::db;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about = "A _very_ simple task management cli")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new task
    Add {
        /// The actual task
        task: String,

        #[arg(short, long)]
        /// Priority [1, 5]. Default 3.
        priority: Option<i64>,
    },

    /// List current tasks
    List {
        #[arg(short, long)]
        /// List all tasks, including completed and cancelled
        all: bool,

        #[arg(long, conflicts_with = "all")]
        /// List completed tasks
        completed: bool,
    },

    /// Mark a task as complete
    Done {
        id: i64,

        #[arg(short, long)]
        /// Also promote next task to "In Progress"
        next: bool,
    },

    /// Automatically choose next task in line
    Next {
        #[arg(short, long)]
        id: Option<i64>,
    },

    /// Show current active task
    #[clap(visible_alias("s"))]
    Show,
}

pub fn run() {
    let args = Cli::parse();
    let conn = db::init_db();
    let active = db::get_current_active_task(&conn);

    match args.command {
        Commands::Add { task, priority } => db::add_task(&conn, &task, priority),

        Commands::List { all, completed } => db::list_tasks(&conn, all, completed),

        Commands::Next { id } => match active {
            None => db::select_next_task(&conn, id),
            Some(_) => {
                println!(
                    "A task is already active.
                    Hint: use `td show` to see current task"
                )
            }
        },

        Commands::Done { id, next } => {
            db::mark_task_done(&conn, id);
            if next && active.is_none() {
                db::select_next_task(&conn, None)
            };
        }

        Commands::Show {} => match active {
            Some(active) => {
                db::print_task_header();
                println!("{active}")
            }
            None => println!(
                "No active task.
                Hint: use `td next` to promote one"
            ),
        },
    }
}
