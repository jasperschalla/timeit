use clap::Parser;

#[derive(Parser)]
#[command(name = "timeit")]
#[command(about = "A CLI tool to track working time", long_about = None)]
pub struct Cli {
    pub action: String,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: i32,
    pub status: String,
    pub start_time: String,
    pub end_time: Option<String>,
}
