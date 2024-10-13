mod models;
mod utils;

use clap::Parser;
use colored::*;
use models::{Cli, Task};
use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use utils::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse arguments
    let args: Cli = Cli::parse();

    // Get current time
    let time_formatted = get_current_time();

    // Create dir for db
    create_dir_if_not_exists()?;

    // Get db path
    let db_path_dir = get_data_dir()?;
    let db_path = PathBuf::from(db_path_dir).join("timeit.db");

    // DB connection
    let conn = Connection::open(db_path)?;

    // Create table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS task (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            status TEXT NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT
        )",
        [],
    )?;

    // Get tasks
    let tasks: Vec<Task> = get_tasks(&conn)?;

    println!();

    match args.action.as_str() {
        "start" => {
            println!(
                "{}",
                "Starting timer. Previous tracking is removed...".green()
            );
            let status = "start";
            conn.execute("DELETE FROM task", params![])?;
            conn.execute("DELETE FROM SQLITE_SEQUENCE WHERE name='task'", params![])?;
            conn.execute(
                "INSERT INTO task (status, start_time) VALUES (?1, ?2)",
                params![status, &time_formatted],
            )?;
            Ok(())
        }
        "stop" => {
            println!("{}", "Stopping timer. Previous tracking is removed...");
            get_status(true, &conn, tasks)?;
            conn.execute("DELETE FROM task", params![])?;
            conn.execute("DELETE FROM SQLITE_SEQUENCE WHERE name='task'", params![])?;
            Ok(())
        }
        "status" => {
            let start = get_start(tasks.clone());
            let last_break = get_last_break(tasks.clone());

            match start {
                Some(_task) => match last_break {
                    Some(task) => {
                        let end_time = task.end_time;

                        match end_time {
                            Some(_) => {
                                println!("Status: {}", "Your are currently working.".green());
                            }
                            None => {
                                println!(
                                    "Status: {}",
                                    "You are currently on a break.".bright_black()
                                );
                            }
                        }
                    }
                    None => {
                        println!("Status: {}", "Your are currently working.".green());
                    }
                },
                None => {
                    println!(
                        "Status: {}",
                        "Your are currently not working.".bright_black()
                    );
                }
            }

            get_status(false, &conn, tasks)?;
            Ok(())
        }
        "break" => {
            println!("{}", "Pausing timer...".bright_black());
            let status = "break";
            let last_break = get_last_break(tasks);

            match last_break {
                Some(task) => {
                    let end_time = task.end_time;

                    match end_time {
                        Some(_time) => {
                            conn.execute(
                                "INSERT INTO task (status, start_time) VALUES (?1, ?2)",
                                params![status, &time_formatted],
                            )?;
                        }
                        None => {
                            println!();
                            println!("{}", "A break is still open...".bright_red());
                            println!(
                                "{}",
                                "Stop the break by using the 'timeit resume' command.".bright_red()
                            );
                        }
                    }
                }
                None => {
                    conn.execute(
                        "INSERT INTO task (status, start_time) VALUES (?1, ?2)",
                        params![status, &time_formatted],
                    )?;
                }
            }
            Ok(())
        }
        "resume" => {
            println!("{}", "Resuming timer...".green());
            let last_break = get_last_break(tasks);

            match last_break {
                Some(task) => {
                    let end_time = task.end_time;

                    match end_time {
                        Some(_time) => {
                            println!();
                            println!("{}", "No open break found...".bright_red());
                            println!(
                                "{}",
                                "Start new break by using the 'timeit break' command.".bright_red()
                            );
                        }
                        None => {
                            let id = task.id;
                            conn.execute(
                                "UPDATE task SET end_time = ?1 WHERE id = ?2",
                                params![&time_formatted, id],
                            )?;
                        }
                    }
                }
                None => {
                    println!();
                    println!("{}", "No break has been started yet...".bright_red());
                    println!(
                        "{}",
                        "Start a new break by using the 'timeit break' command.".bright_red()
                    );
                }
            }
            Ok(())
        }
        _ => {
            println!("{}", "Invalid action.".bright_red());
            Ok(())
        }
    }
}
