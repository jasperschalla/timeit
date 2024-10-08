use chrono::{DateTime, Utc};
use chrono_tz::Europe::Berlin;
use clap::Parser;
use colored::*;
use rusqlite::{params, Connection, Error, Result};

#[derive(Parser)]
#[command(name = "timeit")]
#[command(about = "A CLI tool to track working time", long_about = None)]
struct Cli {
    action: String,
}
#[derive(Debug, Clone)]
struct Task {
    id: i32,
    status: String,
    start_time: String,
    end_time: Option<String>,
}

// Helper

fn get_current_time() -> String {
    let now_utc: DateTime<Utc> = Utc::now();
    let now_berlin = now_utc.with_timezone(&Berlin);
    now_berlin.format("%Y-%m-%d %H:%M:%S %z").to_string()
}

fn get_tasks(conn: &Connection) -> Result<Vec<Task>, Error> {
    let mut stmt = conn.prepare("SELECT * FROM task")?;
    let task_iter = stmt.query_map([], |row| {
        Ok(Task {
            id: row.get(0)?,
            status: row.get(1)?,
            start_time: row.get(2)?,
            end_time: row.get(3)?,
        })
    })?;
    let tasks: Vec<Task> = task_iter.collect::<Result<Vec<Task>, _>>()?;
    Ok(tasks)
}

// Break logic

fn get_last_break(tasks: Vec<Task>) -> Option<Task> {
    // Filter for status break
    let last_break = tasks
        .iter()
        .filter(|task| task.status == "break") // Filter for break tasks
        .max_by_key(|task| task.id.clone()) // Use max_by_key to get the task with the maximum time
        .cloned();

    return last_break;
}

fn get_start(tasks: Vec<Task>) -> Option<Task> {
    // Filter for start
    let start = tasks
        .iter()
        .filter(|task| task.status == "start")
        .max_by_key(|task| task.id.clone())
        .cloned();

    return start;
}

fn compare_break_time(task: &Task) -> f64 {
    //Get start and end time
    let start_time = &task.start_time;
    let end_time = &task.end_time;

    match end_time {
        Some(time) => {
            let start_time_parsed =
                DateTime::parse_from_str(&start_time, "%Y-%m-%d %H:%M:%S %z").unwrap();
            let end_time_parsed = DateTime::parse_from_str(&time, "%Y-%m-%d %H:%M:%S %z").unwrap();

            let duration_sec = end_time_parsed
                .signed_duration_since(start_time_parsed)
                .num_seconds();
            let duration = duration_sec as f64 / 3600.0;
            return duration;
        }
        None => {
            let start_time_parsed =
                DateTime::parse_from_str(&start_time, "%Y-%m-%d %H:%M:%S").unwrap();
            let end_time_parsed =
                DateTime::parse_from_str(&get_current_time(), "%Y-%m-%d %H:%M:%S").unwrap();
            let duration_sec = end_time_parsed
                .signed_duration_since(start_time_parsed)
                .num_seconds();
            let duration = duration_sec as f64 / 60.0;
            return duration;
        }
    }
}

fn get_break_duration(tasks: Vec<Task>) -> f64 {
    let mut duration_counter = 0.0;

    // Filter for status break
    let break_tasks = tasks
        .iter()
        .filter(|task| task.status == "break")
        .cloned()
        .collect::<Vec<Task>>();

    // calculate for each task duration
    for task in break_tasks {
        let duration = compare_break_time(&task);
        duration_counter += duration;
    }

    return duration_counter;
}

// Status logic

fn get_status(end: bool, conn: &Connection, old_tasks: Vec<Task>) -> Result<()> {
    let start_task = get_start(old_tasks.clone());
    let last_break = get_last_break(old_tasks);
    let time = get_current_time();

    if end {
        match last_break {
            Some(task) => {
                let id = task.id;
                let end_time = task.end_time;

                if end_time.is_none() {
                    conn.execute(
                        "UPDATE task SET end_time = ?1 WHERE id = ?2",
                        params![time, id],
                    )?;

                    let start_id = start_task.as_ref().unwrap().id;

                    conn.execute(
                        "UPDATE task SET end_time = ?1 WHERE id = ?2",
                        params![time, start_id],
                    )?;
                }
            }
            None => match &start_task {
                Some(_task) => {
                    let id = start_task.unwrap().id;

                    conn.execute(
                        "UPDATE task SET end_time = ?1 WHERE id = ?2",
                        params![time, id],
                    )?;
                }
                None => {}
            },
        }
    }

    let mut stmt = conn.prepare("SELECT * FROM task")?;
    let task_iter = stmt.query_map([], |row| {
        Ok(Task {
            id: row.get(0)?,
            status: row.get(1)?,
            start_time: row.get(2)?,
            end_time: row.get(3)?,
        })
    })?;
    let tasks: Vec<Task> = task_iter.collect::<Result<Vec<Task>, _>>()?;

    // Print all tasks
    println!();
    println!(
        " {:<5}  {:<8}  {:<26}  {:<14} ",
        "ID", "Status", "Start Time", "End Time"
    );
    println!("-------------------------------------------------------------------------");
    for task in &tasks {
        let message = format!(
            "| {:<4} | {:<6} | {:<25} | {:<25} |",
            task.id,
            task.status,
            task.start_time,
            task.end_time.as_ref().unwrap_or(&"None".to_string())
        );

        match task.status.as_str() {
            "start" => {
                println!("{}", message.green());
            }
            _ => {
                println!("{}", message.black());
            }
        }
        println!("-------------------------------------------------------------------------");
    }

    // Filter for start status and get first element
    let start_task = tasks
        .iter()
        .filter(|task| task.status == "start")
        .next()
        .cloned();

    match end {
        true => {
            // Get first item from task_iter and its time
            match start_task {
                Some(task) => {
                    let start_time_str = task.start_time;

                    let start_time =
                        DateTime::parse_from_str(&start_time_str, "%Y-%m-%d %H:%M:%S %z").unwrap();

                    let now_utc: DateTime<Utc> = Utc::now();
                    let now_berlin = now_utc.with_timezone(&Berlin);

                    let duration_sec = now_berlin.signed_duration_since(start_time).num_seconds();
                    let duration = duration_sec as f64 / 60.0;

                    let break_duration = get_break_duration(tasks.clone());
                    let total_duration = duration - break_duration;

                    println!();
                    println!(
                        "{}",
                        format!("You have finished today's work. Good job! Your worked for {} minute(s).",
                        total_duration.ceil()).green()
                    );
                    return Ok(());
                }
                None => {
                    println!();
                    println!("{}", "You have not started working yet.".black());
                    return Ok(());
                }
            }
        }
        false => {
            return Ok(());
        }
    }
}

fn main() -> Result<()> {
    // Parse arguments
    let args: Cli = Cli::parse();

    // Get current time
    let time_formatted = get_current_time();

    // DB connection
    let conn = Connection::open("timeit.db")?;

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
                                println!("Status: {}", "You are currently on a break.".black());
                            }
                        }
                    }
                    None => {
                        println!("Status: {}", "Your are currently working.".green());
                    }
                },
                None => {
                    println!("Status: {}", "Your are currently not working.".black());
                }
            }

            get_status(false, &conn, tasks)?;
            Ok(())
        }
        "break" => {
            println!("{}", "Pausing timer...".black());
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
