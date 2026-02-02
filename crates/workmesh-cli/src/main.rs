use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand, ValueEnum};

use workmesh_core::backlog::resolve_backlog_dir;
use workmesh_core::task::load_tasks;
use workmesh_core::task_ops::{
    filter_tasks, next_task, render_task_line, sort_tasks, task_to_json_value, tasks_to_json,
};

#[derive(Parser)]
#[command(name = "workmesh", version, about = "WorkMesh CLI (WIP)")]
struct Cli {
    /// Path to repo root or backlog directory
    #[arg(long, required = true)]
    root: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List tasks
    List {
        #[arg(long, action = ArgAction::Append)]
        status: Vec<String>,
        #[arg(long, action = ArgAction::Append)]
        phase: Vec<String>,
        #[arg(long, action = ArgAction::Append)]
        priority: Vec<String>,
        #[arg(long, action = ArgAction::Append, value_name = "label")]
        label: Vec<String>,
        #[arg(long, value_name = "task-id")]
        depends_on: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        deps_satisfied: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        blocked: bool,
        #[arg(long)]
        search: Option<String>,
        #[arg(long, value_enum, default_value_t = SortKey::Id)]
        sort: SortKey,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Show next available task
    Next {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Show a task
    Show {
        task_id: String,
        #[arg(long, action = ArgAction::SetTrue)]
        full: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Show task stats
    Stats {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Export tasks as JSON
    Export {
        #[arg(long, action = ArgAction::SetTrue)]
        pretty: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum SortKey {
    Id,
    Title,
    Status,
    Phase,
    Priority,
}

impl SortKey {
    fn as_str(self) -> &'static str {
        match self {
            SortKey::Id => "id",
            SortKey::Title => "title",
            SortKey::Status => "status",
            SortKey::Phase => "phase",
            SortKey::Priority => "priority",
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let backlog_dir = resolve_backlog_dir(&cli.root)?;
    let tasks = load_tasks(&backlog_dir);

    match cli.command {
        Command::List {
            status,
            phase,
            priority,
            label,
            depends_on,
            deps_satisfied,
            blocked,
            search,
            sort,
            limit,
            json,
        } => {
            let filtered = filter_tasks(
                &tasks,
                to_list(status.as_slice()).as_deref(),
                to_list(phase.as_slice()).as_deref(),
                to_list(priority.as_slice()).as_deref(),
                to_list(label.as_slice()).as_deref(),
                depends_on.as_deref(),
                if deps_satisfied { Some(true) } else { None },
                if blocked { Some(true) } else { None },
                search.as_deref(),
            );
            let mut sorted = sort_tasks(filtered, sort.as_str());
            if let Some(limit) = limit {
                sorted.truncate(limit);
            }
            if json {
                let payload: Vec<_> = sorted.iter().map(|task| (*task).clone()).collect();
                println!("{}", tasks_to_json(&payload, false));
                return Ok(());
            }
            for task in sorted {
                println!("{}", render_task_line(task));
            }
        }
        Command::Next { json } => {
            let task = next_task(&tasks);
            if json {
                if let Some(task) = task {
                    let value = task_to_json_value(&task, false);
                    println!("{}", serde_json::to_string_pretty(&value)?);
                } else {
                    println!("{}", "{}");
                }
            } else if let Some(task) = task {
                println!("{}", render_task_line(&task));
            }
        }
        Command::Show { task_id, full, json } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            if json {
                let value = task_to_json_value(task, true);
                println!("{}", serde_json::to_string_pretty(&value)?);
                return Ok(());
            }
            if full {
                if let Some(path) = &task.file_path {
                    let content = std::fs::read_to_string(path)?;
                    println!("{}", content);
                    return Ok(());
                }
            }
            println!("{}", render_task_line(task));
        }
        Command::Stats { json } => {
            let stats = stats(&tasks);
            if json {
                println!("{}", serde_json::to_string_pretty(&stats)?);
            } else {
                for (key, value) in stats {
                    println!("{}: {}", key, value);
                }
            }
        }
        Command::Export { pretty } => {
            let payload = serde_json::from_str::<serde_json::Value>(&tasks_to_json(&tasks, true))?;
            if pretty {
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("{}", serde_json::to_string(&payload)?);
            }
        }
    }

    Ok(())
}

fn to_list(values: &[String]) -> Option<Vec<String>> {
    let items = split_list(values);
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

fn split_list(values: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for value in values {
        result.extend(split_csv(value));
    }
    result
}

fn split_csv(value: &str) -> Vec<String> {
    if value.trim().is_empty() {
        return Vec::new();
    }
    value
        .split(',')
        .map(|val| val.trim().to_string())
        .filter(|val| !val.is_empty())
        .collect()
}

fn find_task<'a>(tasks: &'a [workmesh_core::task::Task], task_id: &str) -> Option<&'a workmesh_core::task::Task> {
    let target = task_id.to_lowercase();
    tasks.iter().find(|task| task.id.to_lowercase() == target)
}

fn stats(tasks: &[workmesh_core::task::Task]) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    for task in tasks {
        let key = if task.status.is_empty() {
            "(none)".to_string()
        } else {
            task.status.clone()
        };
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

fn die(message: &str) -> ! {
    eprintln!("{}", message);
    std::process::exit(1);
}
