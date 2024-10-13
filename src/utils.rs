use crate::models::Task;
use chrono::{DateTime, Utc};
use chrono_tz::Europe::Berlin;
use colored::*;
use rusqlite::{params, Connection, Error, Result};
use std::env;
use std::fs;
use std::path::PathBuf;

// Helper

pub fn get_data_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home_dir = env::var("HOME")?;
    let folder_path = PathBuf::from(home_dir)
        .join("Library")
        .join("Application Support")
        .join("timeit");
    Ok(folder_path)
}

pub fn create_dir_if_not_exists() -> Result<(), Box<dyn std::error::Error>> {
    let path: PathBuf = get_data_dir()?;
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn get_current_time() -> String {
    let now_utc: DateTime<Utc> = Utc::now();
    let now_berlin = now_utc.with_timezone(&Berlin);
    now_berlin.format("%Y-%m-%d %H:%M:%S %z").to_string()
}

pub fn get_time_delta(task: Task, tasks: Vec<Task>) -> f64 {
    let start_time_str = task.start_time;

    let start_time = DateTime::parse_from_str(&start_time_str, "%Y-%m-%d %H:%M:%S %z").unwrap();

    let now_utc: DateTime<Utc> = Utc::now();
    let now_berlin = now_utc.with_timezone(&Berlin);

    let duration_sec = now_berlin.signed_duration_since(start_time).num_seconds();
    let duration = duration_sec as f64 / 60.0;

    let break_duration = get_break_duration(tasks.clone());
    let total_duration = duration - break_duration;

    return total_duration;
}

// DB helper

pub fn compare_break_time(task: &Task) -> f64 {
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
            let duration = duration_sec as f64 / 60.0;
            return duration;
        }
        None => {
            let start_time_parsed =
                DateTime::parse_from_str(&start_time, "%Y-%m-%d %H:%M:%S %z").unwrap();
            let end_time_parsed =
                DateTime::parse_from_str(&get_current_time(), "%Y-%m-%d %H:%M:%S %z").unwrap();
            let duration_sec = end_time_parsed
                .signed_duration_since(start_time_parsed)
                .num_seconds();
            let duration = duration_sec as f64 / 60.0;
            return duration;
        }
    }
}

pub fn get_break_duration(tasks: Vec<Task>) -> f64 {
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

// DB query

pub fn get_tasks(conn: &Connection) -> Result<Vec<Task>, Error> {
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

pub fn get_last_break(tasks: Vec<Task>) -> Option<Task> {
    // Filter for status break
    let last_break = tasks
        .iter()
        .filter(|task| task.status == "break") // Filter for break tasks
        .max_by_key(|task| task.id.clone()) // Use max_by_key to get the task with the maximum time
        .cloned();

    return last_break;
}

pub fn get_start(tasks: Vec<Task>) -> Option<Task> {
    // Filter for start
    let start = tasks
        .iter()
        .filter(|task| task.status == "start")
        .max_by_key(|task| task.id.clone())
        .cloned();

    return start;
}

// Status print

pub fn get_status(end: bool, conn: &Connection, old_tasks: Vec<Task>) -> Result<()> {
    let start_task = get_start(old_tasks.clone());
    let last_break = get_last_break(old_tasks);
    let time = get_current_time();

    if end {
        match last_break.clone() {
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
                } else {
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
                println!("{}", message.bright_black());
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
                    let total_duration = get_time_delta(task, tasks.clone());

                    let total_duration_hours = (total_duration / 60.0).floor();
                    let total_duration_minutes = (total_duration % 60.0).ceil();

                    println!();
                    println!(
                        "{}",
                        format!("You have finished today's work. Good job! Your worked for {} hour(s) and {} minute(s).",
                        total_duration_hours,
                        total_duration_minutes).green()
                    );
                    return Ok(());
                }
                None => {
                    println!();
                    println!("{}", "You have not started working yet.".bright_black());
                    return Ok(());
                }
            }
        }
        false => match start_task {
            Some(start_task) => {
                match last_break {
                    Some(task) => {
                        let end_time = task.end_time;

                        match end_time {
                            Some(_time) => {
                                let total_duration = get_time_delta(start_task, tasks.clone());

                                let total_duration_hours = (total_duration / 60.0).floor();
                                let total_duration_minutes = (total_duration % 60.0).ceil();

                                println!();
                                println!("{}", format!("You are currently working for {} hour(s) and {} minute(s) so far.",total_duration_hours,total_duration_minutes).green());
                            }
                            None => {
                                let total_duration = get_time_delta(start_task, tasks.clone());

                                let total_duration_hours = (total_duration / 60.0).floor();
                                let total_duration_minutes = (total_duration % 60.0).ceil();
                                println!();
                                println!("{}", format!("You are currently on a break and worked {} hour(s) and {} minute(s) so far.",total_duration_hours,total_duration_minutes).bright_black());
                            }
                        }
                    }
                    None => {
                        let total_duration = get_time_delta(start_task, tasks.clone());

                        let total_duration_hours = (total_duration / 60.0).floor();
                        let total_duration_minutes = (total_duration % 60.0).ceil();

                        println!();
                        println!(
                            "{}",
                            format!(
                                "You are currently working for {} hour(s) and {} minute(s) so far.",
                                total_duration_hours, total_duration_minutes
                            )
                            .green()
                        );
                    }
                }

                return Ok(());
            }
            None => {
                return Ok(());
            }
        },
    }
}
