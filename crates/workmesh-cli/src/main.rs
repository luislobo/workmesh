use std::collections::HashSet;
use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{Duration, Local, NaiveDate};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};

use workmesh_core::archive::{archive_tasks, ArchiveOptions};
use workmesh_core::audit::{append_audit_event, AuditEvent};
use workmesh_core::backlog::{resolve_backlog, BacklogResolution};
use workmesh_core::config::update_do_not_migrate;
use workmesh_core::gantt::{
    plantuml_gantt, render_plantuml_svg, write_text_file, PlantumlRenderError,
};
use workmesh_core::index::{rebuild_index, refresh_index, verify_index};
use workmesh_core::migration::{migrate_backlog, MigrationError};
use workmesh_core::project::{ensure_project_docs, repo_root_from_backlog};
use workmesh_core::quickstart::quickstart;
use workmesh_core::session::{
    append_session_journal, diff_since_checkpoint, render_diff, render_resume, resolve_project_id,
    resume_summary, task_summary, write_checkpoint, write_working_set, CheckpointOptions,
};
use workmesh_core::task::{load_tasks, Lease, Task};
use workmesh_core::task_ops::{
    append_note, create_task_file, filter_tasks, graph_export, next_task, now_timestamp,
    ready_tasks, render_task_line, replace_section, set_list_field, sort_tasks, status_counts,
    task_to_json_value, tasks_to_json, tasks_to_jsonl, timestamp_plus_minutes, update_body,
    update_lease_fields, update_task_field, update_task_field_or_section, validate_tasks,
    FieldValue,
};

#[derive(Parser)]
#[command(name = "workmesh", version, about = "WorkMesh CLI (WIP)")]
struct Cli {
    /// Path to repo root or backlog directory
    #[arg(long, required = true)]
    root: PathBuf,
    /// Automatically write a checkpoint after mutating commands
    #[arg(long, action = ArgAction::SetTrue, global = true)]
    auto_checkpoint: bool,
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
        kind: Vec<String>,
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
    /// List ready tasks
    Ready {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
        #[arg(long)]
        limit: Option<usize>,
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
    /// Export task graph as JSON
    GraphExport {
        #[arg(long, action = ArgAction::SetTrue)]
        pretty: bool,
    },
    /// Export tasks as JSON
    Export {
        #[arg(long, action = ArgAction::SetTrue)]
        pretty: bool,
    },
    /// Export tasks as JSONL
    IssuesExport {
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long, action = ArgAction::SetTrue)]
        include_body: bool,
    },
    /// Rebuild JSONL task index
    IndexRebuild {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Refresh JSONL task index
    IndexRefresh {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Verify JSONL task index
    IndexVerify {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Write a session checkpoint (JSON + Markdown)
    Checkpoint {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        audit_limit: Option<usize>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Resume from the latest checkpoint
    Resume {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        id: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Write the working set file
    WorkingSet {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        tasks: Option<String>,
        #[arg(long)]
        note: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Append a session journal entry
    SessionJournal {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        next: Option<String>,
        #[arg(long)]
        note: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Show changes since a checkpoint
    CheckpointDiff {
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        id: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Set task status
    SetStatus {
        task_id: String,
        status: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
    },
    /// Claim a task (lease)
    Claim {
        task_id: String,
        owner: String,
        #[arg(long)]
        minutes: Option<i64>,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
    },
    /// Release a task lease
    Release {
        task_id: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
    },
    /// Bulk operations (alias group)
    Bulk {
        #[command(subcommand)]
        command: BulkCommand,
    },
    /// Bulk set status for tasks
    BulkSetStatus {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        status: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Bulk set a front matter field for tasks
    BulkSetField {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        field: String,
        #[arg(long)]
        value: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Bulk add label to tasks
    BulkLabelAdd {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        label: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Bulk remove label from tasks
    BulkLabelRemove {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        label: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Bulk add dependency to tasks
    BulkDepAdd {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        dependency: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Bulk remove dependency from tasks
    BulkDepRemove {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        dependency: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Bulk append a note to tasks
    BulkNote {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        note: String,
        #[arg(long, value_enum, default_value_t = NoteSection::Notes)]
        section: NoteSection,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Set a front matter field
    SetField {
        task_id: String,
        field: String,
        value: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
    },
    /// Add label to task
    LabelAdd {
        task_id: String,
        label: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
    },
    /// Remove label from task
    LabelRemove {
        task_id: String,
        label: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
    },
    /// Add dependency to task
    DepAdd {
        task_id: String,
        dependency: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
    },
    /// Remove dependency from task
    DepRemove {
        task_id: String,
        dependency: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
    },
    /// Append a note to a task
    Note {
        task_id: String,
        note: String,
        #[arg(long, value_enum, default_value_t = NoteSection::Notes)]
        section: NoteSection,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
    },
    /// Replace task body (all content after front matter)
    SetBody {
        task_id: String,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
    },
    /// Replace a named section in the task body
    SetSection {
        task_id: String,
        section: String,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
    },
    /// Create a new task
    Add {
        #[arg(long, value_name = "task-id")]
        id: Option<String>,
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "To Do")]
        status: String,
        #[arg(long, default_value = "P2")]
        priority: String,
        #[arg(long, default_value = "Phase1")]
        phase: String,
        #[arg(long, default_value = "")]
        labels: String,
        #[arg(long, default_value = "")]
        dependencies: String,
        #[arg(long, default_value = "")]
        assignee: String,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Create a task discovered from another task
    AddDiscovered {
        #[arg(long)]
        from: String,
        #[arg(long, value_name = "task-id")]
        id: Option<String>,
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "To Do")]
        status: String,
        #[arg(long, default_value = "P2")]
        priority: String,
        #[arg(long, default_value = "Phase1")]
        phase: String,
        #[arg(long, default_value = "")]
        labels: String,
        #[arg(long, default_value = "")]
        dependencies: String,
        #[arg(long, default_value = "")]
        assignee: String,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Create project docs scaffold
    ProjectInit {
        project_id: String,
        #[arg(long)]
        name: Option<String>,
    },
    /// Quickstart: scaffold docs + backlog + seed task
    Quickstart {
        project_id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        agents_snippet: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Migrate legacy backlog layout to workmesh/
    Migrate {
        #[arg(long)]
        to: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        yes: bool,
    },
    /// Archive done tasks into date-based folders
    Archive {
        #[arg(long, default_value = "30d")]
        before: String,
        #[arg(long, default_value = "Done")]
        status: String,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Validate task files
    Validate {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Show backlog best practices
    BestPractices,
    /// Render PlantUML gantt text
    Gantt {
        #[arg(long)]
        start: Option<String>,
        #[arg(long, default_value_t = 3)]
        zoom: i32,
    },
    /// Write PlantUML gantt to a file
    GanttFile {
        #[arg(long)]
        start: Option<String>,
        #[arg(long, default_value_t = 3)]
        zoom: i32,
        #[arg(long)]
        output: PathBuf,
    },
    /// Render gantt SVG via PlantUML
    GanttSvg {
        #[arg(long)]
        start: Option<String>,
        #[arg(long, default_value_t = 3)]
        zoom: i32,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        plantuml_cmd: Option<String>,
        #[arg(long)]
        plantuml_jar: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum BulkCommand {
    /// Bulk set status for tasks
    SetStatus {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        status: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Bulk set a front matter field for tasks
    SetField {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        field: String,
        #[arg(long)]
        value: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Bulk add label to tasks
    LabelAdd {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        label: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Bulk remove label from tasks
    LabelRemove {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        label: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Bulk add dependency to tasks
    DepAdd {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        dependency: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Bulk remove dependency from tasks
    DepRemove {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        dependency: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Bulk append a note to tasks
    Note {
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        tasks: Vec<String>,
        #[arg(long)]
        note: String,
        #[arg(long, value_enum, default_value_t = NoteSection::Notes)]
        section: NoteSection,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum SortKey {
    Id,
    Title,
    Kind,
    Status,
    Phase,
    Priority,
}

impl SortKey {
    fn as_str(self) -> &'static str {
        match self {
            SortKey::Id => "id",
            SortKey::Title => "title",
            SortKey::Kind => "kind",
            SortKey::Status => "status",
            SortKey::Phase => "phase",
            SortKey::Priority => "priority",
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum NoteSection {
    Notes,
    Impl,
}

impl NoteSection {
    fn as_str(self) -> &'static str {
        match self {
            NoteSection::Notes => "notes",
            NoteSection::Impl => "impl",
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Command::Quickstart {
        project_id,
        name,
        agents_snippet,
        json,
    } = &cli.command
    {
        let repo_root = repo_root_from_backlog(&cli.root);
        let result = quickstart(&repo_root, project_id, name.as_deref(), *agents_snippet)?;
        if *json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("Docs: {}", result.project_dir.display());
            println!("WorkMesh: {}", result.backlog_dir.display());
            if let Some(task_path) = result.created_task.as_ref() {
                println!("Seed task: {}", task_path.display());
            }
            if result.agents_snippet_written {
                println!("AGENTS.md updated");
            }
        }
        return Ok(());
    }

    if let Command::Migrate { to, yes } = &cli.command {
        let resolution = resolve_backlog(&cli.root)?;
        let target = to.as_deref().unwrap_or("workmesh");
        handle_migrate_command(&resolution, target, *yes)?;
        return Ok(());
    }

    let resolution = resolve_backlog(&cli.root)?;
    let backlog_dir = maybe_prompt_migration(&resolution)?;
    let tasks = load_tasks(&backlog_dir);
    let auto_checkpoint = auto_checkpoint_enabled(&cli);

    match cli.command {
        Command::List {
            status,
            kind,
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
                to_list(kind.as_slice()).as_deref(),
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
        Command::Ready { json, limit } => {
            let mut ready = ready_tasks(&tasks);
            if let Some(limit) = limit {
                ready.truncate(limit);
            }
            if json {
                let payload: Vec<_> = ready.iter().map(|task| (*task).clone()).collect();
                println!("{}", tasks_to_json(&payload, false));
                return Ok(());
            }
            for task in ready {
                println!("{}", render_task_line(task));
            }
        }
        Command::Show {
            task_id,
            full,
            json,
        } => {
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
            let stats = status_counts(&tasks);
            if json {
                let mut map = serde_json::Map::new();
                for (key, value) in stats {
                    map.insert(key, serde_json::Value::from(value as u64));
                }
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::Value::Object(map))?
                );
            } else {
                for (key, value) in stats {
                    println!("{}: {}", key, value);
                }
            }
        }
        Command::GraphExport { pretty } => {
            let graph = graph_export(&tasks);
            if pretty {
                println!("{}", serde_json::to_string_pretty(&graph)?);
            } else {
                println!("{}", serde_json::to_string(&graph)?);
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
        Command::IssuesExport {
            output,
            include_body,
        } => {
            let payload = tasks_to_jsonl(&tasks, include_body);
            if let Some(output) = output {
                std::fs::write(&output, payload)?;
                println!("{}", output.display());
            } else {
                println!("{}", payload);
            }
        }
        Command::IndexRebuild { json } => {
            let summary = rebuild_index(&backlog_dir)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                println!(
                    "Index rebuild -> {} ({} entries)",
                    summary.path, summary.entries
                );
            }
        }
        Command::IndexRefresh { json } => {
            let summary = refresh_index(&backlog_dir)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                println!(
                    "Index refresh -> {} ({} entries)",
                    summary.path, summary.entries
                );
            }
        }
        Command::IndexVerify { json } => {
            let report = verify_index(&backlog_dir)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if report.ok {
                println!("Index ok");
            } else {
                if !report.missing.is_empty() {
                    println!("Missing: {}", report.missing.len());
                }
                if !report.stale.is_empty() {
                    println!("Stale: {}", report.stale.len());
                }
                if !report.extra.is_empty() {
                    println!("Extra: {}", report.extra.len());
                }
                std::process::exit(1);
            }
        }
        Command::Checkpoint {
            project,
            id,
            audit_limit,
            json,
        } => {
            let options = CheckpointOptions {
                project_id: project.clone(),
                checkpoint_id: id.clone(),
                audit_limit: audit_limit.unwrap_or(20),
            };
            let result = write_checkpoint(&backlog_dir, &tasks, &options)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result.snapshot)?);
            } else {
                println!("Checkpoint: {}", result.snapshot.checkpoint_id);
                println!("JSON: {}", result.json_path.display());
                println!("Markdown: {}", result.markdown_path.display());
            }
        }
        Command::Resume { project, id, json } => {
            let repo_root = repo_root_from_backlog(&backlog_dir);
            let project_id = resolve_project_id(&repo_root, &tasks, project.as_deref());
            let summary = resume_summary(&repo_root, &project_id, id.as_deref())?;
            match summary {
                Some(summary) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&summary.snapshot)?);
                    } else {
                        println!("{}", render_resume(&summary));
                    }
                }
                None => {
                    println!("No checkpoint found");
                }
            }
        }
        Command::WorkingSet {
            project,
            tasks: task_list,
            note,
            json,
        } => {
            let repo_root = repo_root_from_backlog(&backlog_dir);
            let project_id = resolve_project_id(&repo_root, &tasks, project.as_deref());
            let selected = match task_list.as_deref() {
                Some(list) if !list.trim().is_empty() => {
                    let ids = split_csv(list);
                    select_tasks_by_ids(&tasks, &ids)
                }
                _ => tasks
                    .iter()
                    .filter(|task| task.status.eq_ignore_ascii_case("in progress"))
                    .collect(),
            };
            let summaries: Vec<_> = selected.iter().map(|task| task_summary(task)).collect();
            let path = write_working_set(&repo_root, &project_id, &summaries, note.as_deref())?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({"path": path}))?
                );
            } else {
                println!("{}", path.display());
            }
        }
        Command::SessionJournal {
            project,
            task,
            next,
            note,
            json,
        } => {
            let repo_root = repo_root_from_backlog(&backlog_dir);
            let project_id = resolve_project_id(&repo_root, &tasks, project.as_deref());
            let path = append_session_journal(
                &repo_root,
                &project_id,
                task.as_deref(),
                next.as_deref(),
                note.as_deref(),
            )?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({"path": path}))?
                );
            } else {
                println!("{}", path.display());
            }
        }
        Command::CheckpointDiff { project, id, json } => {
            let repo_root = repo_root_from_backlog(&backlog_dir);
            let project_id = resolve_project_id(&repo_root, &tasks, project.as_deref());
            let summary = resume_summary(&repo_root, &project_id, id.as_deref())?;
            let Some(summary) = summary else {
                println!("No checkpoint found");
                return Ok(());
            };
            let report = diff_since_checkpoint(&repo_root, &backlog_dir, &tasks, &summary.snapshot);
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", render_diff(&report));
            }
        }
        Command::SetStatus {
            task_id,
            status,
            touch,
        } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            update_task_field(path, "status", Some(status.clone().into()))?;
            if touch {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
            }
            audit_event(
                &backlog_dir,
                "set_status",
                Some(&task.id),
                serde_json::json!({ "status": status.clone() }),
            )?;
            refresh_index_best_effort(&backlog_dir);
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
            println!("Updated {} status -> {}", task.id, status);
        }
        Command::Claim {
            task_id,
            owner,
            minutes,
            touch,
        } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let mut assignee = task.assignee.clone();
            if !assignee.iter().any(|value| value == &owner) {
                assignee.push(owner.clone());
                set_list_field(path, "assignee", assignee)?;
            }
            let expires_at = minutes.map(timestamp_plus_minutes);
            let lease = Lease {
                owner,
                acquired_at: Some(now_timestamp()),
                expires_at,
            };
            update_lease_fields(path, Some(&lease))?;
            if touch {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
            }
            audit_event(
                &backlog_dir,
                "claim",
                Some(&task.id),
                serde_json::json!({
                    "owner": lease.owner.clone(),
                    "expires_at": lease.expires_at.clone(),
                }),
            )?;
            refresh_index_best_effort(&backlog_dir);
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
            println!("Claimed {} lease -> {}", task.id, lease.owner);
        }
        Command::Release { task_id, touch } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            update_lease_fields(path, None)?;
            if touch {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
            }
            audit_event(
                &backlog_dir,
                "release",
                Some(&task.id),
                serde_json::json!({}),
            )?;
            refresh_index_best_effort(&backlog_dir);
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
            println!("Released {} lease", task.id);
        }
        Command::Bulk { command } => match command {
            BulkCommand::SetStatus {
                tasks: task_ids,
                status,
                touch,
                json,
            } => handle_bulk_set_status(
                &backlog_dir,
                &tasks,
                task_ids,
                status,
                touch,
                json,
                auto_checkpoint,
            )?,
            BulkCommand::SetField {
                tasks: task_ids,
                field,
                value,
                touch,
                json,
            } => handle_bulk_set_field(
                &backlog_dir,
                &tasks,
                task_ids,
                field,
                value,
                touch,
                json,
                auto_checkpoint,
            )?,
            BulkCommand::LabelAdd {
                tasks: task_ids,
                label,
                touch,
                json,
            } => handle_bulk_label_add(
                &backlog_dir,
                &tasks,
                task_ids,
                label,
                touch,
                json,
                auto_checkpoint,
            )?,
            BulkCommand::LabelRemove {
                tasks: task_ids,
                label,
                touch,
                json,
            } => handle_bulk_label_remove(
                &backlog_dir,
                &tasks,
                task_ids,
                label,
                touch,
                json,
                auto_checkpoint,
            )?,
            BulkCommand::DepAdd {
                tasks: task_ids,
                dependency,
                touch,
                json,
            } => handle_bulk_dep_add(
                &backlog_dir,
                &tasks,
                task_ids,
                dependency,
                touch,
                json,
                auto_checkpoint,
            )?,
            BulkCommand::DepRemove {
                tasks: task_ids,
                dependency,
                touch,
                json,
            } => handle_bulk_dep_remove(
                &backlog_dir,
                &tasks,
                task_ids,
                dependency,
                touch,
                json,
                auto_checkpoint,
            )?,
            BulkCommand::Note {
                tasks: task_ids,
                note,
                section,
                touch,
                json,
            } => handle_bulk_note(
                &backlog_dir,
                &tasks,
                task_ids,
                note,
                section,
                touch,
                json,
                auto_checkpoint,
            )?,
        },
        Command::BulkSetStatus {
            tasks: task_ids,
            status,
            touch,
            json,
        } => {
            handle_bulk_set_status(
                &backlog_dir,
                &tasks,
                task_ids,
                status,
                touch,
                json,
                auto_checkpoint,
            )?;
        }
        Command::BulkSetField {
            tasks: task_ids,
            field,
            value,
            touch,
            json,
        } => {
            handle_bulk_set_field(
                &backlog_dir,
                &tasks,
                task_ids,
                field,
                value,
                touch,
                json,
                auto_checkpoint,
            )?;
        }
        Command::BulkLabelAdd {
            tasks: task_ids,
            label,
            touch,
            json,
        } => {
            handle_bulk_label_add(
                &backlog_dir,
                &tasks,
                task_ids,
                label,
                touch,
                json,
                auto_checkpoint,
            )?;
        }
        Command::BulkLabelRemove {
            tasks: task_ids,
            label,
            touch,
            json,
        } => {
            handle_bulk_label_remove(
                &backlog_dir,
                &tasks,
                task_ids,
                label,
                touch,
                json,
                auto_checkpoint,
            )?;
        }
        Command::BulkDepAdd {
            tasks: task_ids,
            dependency,
            touch,
            json,
        } => {
            handle_bulk_dep_add(
                &backlog_dir,
                &tasks,
                task_ids,
                dependency,
                touch,
                json,
                auto_checkpoint,
            )?;
        }
        Command::BulkDepRemove {
            tasks: task_ids,
            dependency,
            touch,
            json,
        } => {
            handle_bulk_dep_remove(
                &backlog_dir,
                &tasks,
                task_ids,
                dependency,
                touch,
                json,
                auto_checkpoint,
            )?;
        }
        Command::BulkNote {
            tasks: task_ids,
            note,
            section,
            touch,
            json,
        } => {
            handle_bulk_note(
                &backlog_dir,
                &tasks,
                task_ids,
                note,
                section,
                touch,
                json,
                auto_checkpoint,
            )?;
        }
        Command::SetField {
            task_id,
            field,
            value,
            touch,
        } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            update_task_field_or_section(path, &field, Some(&value))?;
            if touch {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
            }
            audit_event(
                &backlog_dir,
                "set_field",
                Some(&task.id),
                serde_json::json!({ "field": field.clone(), "value": value.clone() }),
            )?;
            refresh_index_best_effort(&backlog_dir);
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
            println!("Updated {} {} -> {}", task.id, field, value);
        }
        Command::LabelAdd {
            task_id,
            label,
            touch,
        } => {
            update_list_field(
                &backlog_dir,
                &tasks,
                &task_id,
                "labels",
                &label,
                true,
                touch,
            )?;
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
        }
        Command::LabelRemove {
            task_id,
            label,
            touch,
        } => {
            update_list_field(
                &backlog_dir,
                &tasks,
                &task_id,
                "labels",
                &label,
                false,
                touch,
            )?;
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
        }
        Command::DepAdd {
            task_id,
            dependency,
            touch,
        } => {
            update_list_field(
                &backlog_dir,
                &tasks,
                &task_id,
                "dependencies",
                &dependency,
                true,
                touch,
            )?;
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
        }
        Command::DepRemove {
            task_id,
            dependency,
            touch,
        } => {
            update_list_field(
                &backlog_dir,
                &tasks,
                &task_id,
                "dependencies",
                &dependency,
                false,
                touch,
            )?;
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
        }
        Command::Note {
            task_id,
            note,
            section,
            touch,
        } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let new_body = append_note(&task.body, &note, section.as_str());
            update_body(path, &new_body)?;
            if touch {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
            }
            audit_event(
                &backlog_dir,
                "note",
                Some(&task.id),
                serde_json::json!({ "section": section.as_str(), "note": note }),
            )?;
            refresh_index_best_effort(&backlog_dir);
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
            println!("Added note to {}", task.id);
        }
        Command::SetBody {
            task_id,
            text,
            file,
            touch,
        } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let content = read_content(text.as_deref(), file.as_deref())?;
            update_body(path, &content)?;
            if touch {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
            }
            audit_event(
                &backlog_dir,
                "set_body",
                Some(&task.id),
                serde_json::json!({ "length": content.len() }),
            )?;
            refresh_index_best_effort(&backlog_dir);
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
            println!("Updated body for {}", task.id);
        }
        Command::SetSection {
            task_id,
            section,
            text,
            file,
            touch,
        } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let content = read_content(text.as_deref(), file.as_deref())?;
            let new_body = replace_section(&task.body, &section, &content);
            update_body(path, &new_body)?;
            if touch {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
            }
            audit_event(
                &backlog_dir,
                "set_section",
                Some(&task.id),
                serde_json::json!({ "section": section.clone(), "length": content.len() }),
            )?;
            refresh_index_best_effort(&backlog_dir);
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
            println!("Updated section {} for {}", section, task.id);
        }
        Command::Add {
            id,
            title,
            status,
            priority,
            phase,
            labels,
            dependencies,
            assignee,
            json,
        } => {
            let tasks_dir = backlog_dir.join("tasks");
            let task_id = id.unwrap_or_else(|| next_id(&tasks));
            let labels = split_csv(&labels);
            let dependencies = split_csv(&dependencies);
            let assignee = split_csv(&assignee);
            let path = create_task_file(
                &tasks_dir,
                &task_id,
                &title,
                &status,
                &priority,
                &phase,
                &dependencies,
                &labels,
                &assignee,
            )?;
            audit_event(
                &backlog_dir,
                "add_task",
                Some(&task_id),
                serde_json::json!({ "title": title }),
            )?;
            refresh_index_best_effort(&backlog_dir);
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
            if json {
                let payload = serde_json::json!({"path": path, "id": task_id});
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("Created {} -> {}", task_id, path.display());
            }
        }
        Command::AddDiscovered {
            from,
            id,
            title,
            status,
            priority,
            phase,
            labels,
            dependencies,
            assignee,
            json,
        } => {
            let tasks_dir = backlog_dir.join("tasks");
            let task_id = id.unwrap_or_else(|| next_id(&tasks));
            let labels = split_csv(&labels);
            let dependencies = split_csv(&dependencies);
            let assignee = split_csv(&assignee);
            let path = create_task_file(
                &tasks_dir,
                &task_id,
                &title,
                &status,
                &priority,
                &phase,
                &dependencies,
                &labels,
                &assignee,
            )?;
            update_task_field(
                &path,
                "discovered_from",
                Some(FieldValue::List(vec![from.clone()])),
            )?;
            audit_event(
                &backlog_dir,
                "add_discovered",
                Some(&task_id),
                serde_json::json!({ "from": from, "title": title }),
            )?;
            refresh_index_best_effort(&backlog_dir);
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
            if json {
                let payload = serde_json::json!({"path": path, "id": task_id});
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("Created {} -> {}", task_id, path.display());
            }
        }
        Command::ProjectInit { project_id, name } => {
            let repo_root = repo_root_from_backlog(&backlog_dir);
            let path = ensure_project_docs(&repo_root, &project_id, name.as_deref())?;
            audit_event(
                &backlog_dir,
                "project_init",
                None,
                serde_json::json!({ "project_id": project_id }),
            )?;
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
            println!("{}", path.display());
        }
        Command::Validate { json } => {
            let report = validate_tasks(&tasks, Some(&backlog_dir));
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                for err in &report.errors {
                    println!("ERROR: {}", err);
                }
                for warn in &report.warnings {
                    println!("WARN: {}", warn);
                }
                if !report.errors.is_empty() {
                    std::process::exit(1);
                }
            }
        }
        Command::BestPractices => {
            println!("{}", best_practices_text());
        }
        Command::Gantt { start, zoom } => {
            let text = plantuml_gantt(&tasks, start.as_deref(), None, zoom, None, true);
            print!("{}", text);
        }
        Command::GanttFile {
            start,
            zoom,
            output,
        } => {
            let text = plantuml_gantt(&tasks, start.as_deref(), None, zoom, None, true);
            let path = write_text_file(&output, &text)?;
            println!("{}", path.display());
        }
        Command::GanttSvg {
            start,
            zoom,
            output,
            plantuml_cmd,
            plantuml_jar,
        } => {
            let text = plantuml_gantt(&tasks, start.as_deref(), None, zoom, None, true);
            let cmd = match plantuml_cmd {
                Some(cmd) => Some(shell_words::split(&cmd).map_err(anyhow::Error::msg)?),
                None => None,
            };
            let svg =
                render_plantuml_svg(&text, cmd, plantuml_jar.as_deref(), None).map_err(|err| {
                    match err {
                        PlantumlRenderError::RenderFailed(msg) => anyhow::Error::msg(msg),
                        other => anyhow::Error::msg(other.to_string()),
                    }
                })?;
            if let Some(output) = output {
                let path = write_text_file(&output, &svg)?;
                println!("{}", path.display());
            } else {
                print!("{}", svg);
            }
        }
        Command::Quickstart { .. } => {
            unreachable!("quickstart handled before backlog resolution");
        }
        Command::Migrate { .. } => {
            unreachable!("migrate handled before backlog resolution");
        }
        Command::Archive {
            before,
            status,
            json,
        } => {
            let before_date = parse_before_date(&before)?;
            let result = archive_tasks(
                &backlog_dir,
                &tasks,
                &ArchiveOptions {
                    before: before_date,
                    status: status.clone(),
                },
            )?;
            refresh_index_best_effort(&backlog_dir);
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint);
            if json {
                let payload = serde_json::json!({
                    "archived": result.archived,
                    "skipped": result.skipped,
                    "archive_dir": result.archive_dir
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("Archived {} tasks", result.archived.len());
                if !result.skipped.is_empty() {
                    println!("Skipped: {}", result.skipped.join(", "));
                }
                println!("Archive: {}", result.archive_dir.display());
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

fn find_task<'a>(tasks: &'a [Task], task_id: &str) -> Option<&'a Task> {
    let target = task_id.to_lowercase();
    tasks.iter().find(|task| task.id.to_lowercase() == target)
}

fn select_tasks_by_ids<'a>(tasks: &'a [Task], ids: &[String]) -> Vec<&'a Task> {
    let mut selected = Vec::new();
    for id in ids {
        if let Some(task) = find_task(tasks, id) {
            selected.push(task);
        }
    }
    selected
}

fn select_tasks_with_missing<'a>(
    tasks: &'a [Task],
    ids: &[String],
) -> (Vec<&'a Task>, Vec<String>) {
    let mut selected = Vec::new();
    let mut missing = Vec::new();
    for id in ids {
        if let Some(task) = find_task(tasks, id) {
            selected.push(task);
        } else {
            missing.push(id.to_string());
        }
    }
    (selected, missing)
}

fn normalize_task_ids(ids: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for id in ids {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            continue;
        }
        let key = trimmed.to_lowercase();
        if seen.insert(key) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn emit_bulk_result(updated: &[String], missing: &[String], json: bool) {
    let payload = serde_json::json!({
        "ok": missing.is_empty(),
        "updated": updated,
        "missing": missing,
    });
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_default()
        );
    } else {
        println!("Updated {} tasks", updated.len());
        if !missing.is_empty() {
            println!("Missing tasks: {}", missing.join(", "));
        }
    }
    if !missing.is_empty() {
        std::process::exit(1);
    }
}

fn parse_before_date(value: &str) -> Result<NaiveDate> {
    let trimmed = value.trim();
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Ok(date);
    }
    if let Some(days) = trimmed.strip_suffix('d') {
        if let Ok(days) = days.parse::<i64>() {
            let date = Local::now().date_naive() - Duration::days(days);
            return Ok(date);
        }
    }
    Err(anyhow::anyhow!("Invalid date format: {}", value))
}

fn maybe_prompt_migration(resolution: &BacklogResolution) -> Result<PathBuf> {
    if !resolution.layout.is_legacy() {
        return Ok(resolution.backlog_dir.clone());
    }
    if resolution
        .config
        .as_ref()
        .and_then(|cfg| cfg.do_not_migrate)
        .unwrap_or(false)
    {
        return Ok(resolution.backlog_dir.clone());
    }
    if !io::stdin().is_terminal() {
        eprintln!(
            "Legacy backlog detected at {}. Run `workmesh --root . migrate` to move to workmesh/.",
            resolution.backlog_dir.display()
        );
        return Ok(resolution.backlog_dir.clone());
    }
    if confirm_migration(&resolution.backlog_dir)? {
        let result = migrate_backlog(resolution, "workmesh")?;
        let _ = update_do_not_migrate(&resolution.repo_root, false);
        return Ok(result.to);
    }
    let _ = update_do_not_migrate(&resolution.repo_root, true);
    Ok(resolution.backlog_dir.clone())
}

fn confirm_migration(path: &Path) -> Result<bool> {
    eprint!(
        "Legacy backlog found at {}. Migrate to workmesh/? [y/N] ",
        path.display()
    );
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let value = input.trim().to_lowercase();
    Ok(matches!(value.as_str(), "y" | "yes"))
}

fn handle_migrate_command(resolution: &BacklogResolution, target: &str, yes: bool) -> Result<()> {
    if !yes && io::stdin().is_terminal() {
        if !confirm_migration(&resolution.backlog_dir)? {
            let _ = update_do_not_migrate(&resolution.repo_root, true);
            println!("Migration cancelled.");
            return Ok(());
        }
    }
    match migrate_backlog(resolution, target) {
        Ok(result) => {
            let _ = update_do_not_migrate(&resolution.repo_root, false);
            println!(
                "Migrated {} -> {}",
                result.from.display(),
                result.to.display()
            );
        }
        Err(MigrationError::AlreadyMigrated(path)) => {
            println!("Already migrated at {}", path.display());
        }
        Err(MigrationError::DestinationExists(path)) => {
            println!("Destination exists: {}", path.display());
        }
        Err(err) => return Err(err.into()),
    }
    Ok(())
}

fn handle_bulk_set_status(
    backlog_dir: &Path,
    tasks: &[Task],
    task_ids: Vec<String>,
    status: String,
    touch: bool,
    json: bool,
    auto_checkpoint: bool,
) -> Result<()> {
    let ids = normalize_task_ids(split_list(&task_ids));
    if ids.is_empty() {
        die("No tasks provided");
    }
    let (selected, missing) = select_tasks_with_missing(tasks, &ids);
    let mut updated = Vec::new();
    for task in selected {
        let path = task.file_path.as_ref().unwrap_or_else(|| {
            die(&format!("Task not found: {}", task.id));
        });
        update_task_field(path, "status", Some(FieldValue::Scalar(status.clone())))?;
        if touch {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
        }
        audit_event(
            backlog_dir,
            "bulk_set_status",
            Some(&task.id),
            serde_json::json!({ "status": status.clone() }),
        )?;
        updated.push(task.id.clone());
    }
    refresh_index_best_effort(backlog_dir);
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint);
    emit_bulk_result(&updated, &missing, json);
    Ok(())
}

fn handle_bulk_set_field(
    backlog_dir: &Path,
    tasks: &[Task],
    task_ids: Vec<String>,
    field: String,
    value: String,
    touch: bool,
    json: bool,
    auto_checkpoint: bool,
) -> Result<()> {
    let ids = normalize_task_ids(split_list(&task_ids));
    if ids.is_empty() {
        die("No tasks provided");
    }
    let (selected, missing) = select_tasks_with_missing(tasks, &ids);
    let mut updated = Vec::new();
    for task in selected {
        let path = task.file_path.as_ref().unwrap_or_else(|| {
            die(&format!("Task not found: {}", task.id));
        });
        update_task_field_or_section(path, &field, Some(&value))?;
        if touch {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
        }
        audit_event(
            backlog_dir,
            "bulk_set_field",
            Some(&task.id),
            serde_json::json!({ "field": field.clone(), "value": value.clone() }),
        )?;
        updated.push(task.id.clone());
    }
    refresh_index_best_effort(backlog_dir);
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint);
    emit_bulk_result(&updated, &missing, json);
    Ok(())
}

fn handle_bulk_label_add(
    backlog_dir: &Path,
    tasks: &[Task],
    task_ids: Vec<String>,
    label: String,
    touch: bool,
    json: bool,
    auto_checkpoint: bool,
) -> Result<()> {
    let ids = normalize_task_ids(split_list(&task_ids));
    if ids.is_empty() {
        die("No tasks provided");
    }
    let (selected, missing) = select_tasks_with_missing(tasks, &ids);
    let mut updated = Vec::new();
    for task in selected {
        let path = task.file_path.as_ref().unwrap_or_else(|| {
            die(&format!("Task not found: {}", task.id));
        });
        let mut current = task.labels.clone();
        if !current.contains(&label) {
            current.push(label.clone());
        }
        set_list_field(path, "labels", current)?;
        if touch {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
        }
        audit_event(
            backlog_dir,
            "bulk_label_add",
            Some(&task.id),
            serde_json::json!({ "label": label.clone() }),
        )?;
        updated.push(task.id.clone());
    }
    refresh_index_best_effort(backlog_dir);
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint);
    emit_bulk_result(&updated, &missing, json);
    Ok(())
}

fn handle_bulk_label_remove(
    backlog_dir: &Path,
    tasks: &[Task],
    task_ids: Vec<String>,
    label: String,
    touch: bool,
    json: bool,
    auto_checkpoint: bool,
) -> Result<()> {
    let ids = normalize_task_ids(split_list(&task_ids));
    if ids.is_empty() {
        die("No tasks provided");
    }
    let (selected, missing) = select_tasks_with_missing(tasks, &ids);
    let mut updated = Vec::new();
    for task in selected {
        let path = task.file_path.as_ref().unwrap_or_else(|| {
            die(&format!("Task not found: {}", task.id));
        });
        let mut current = task.labels.clone();
        current.retain(|entry| entry != &label);
        set_list_field(path, "labels", current)?;
        if touch {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
        }
        audit_event(
            backlog_dir,
            "bulk_label_remove",
            Some(&task.id),
            serde_json::json!({ "label": label.clone() }),
        )?;
        updated.push(task.id.clone());
    }
    refresh_index_best_effort(backlog_dir);
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint);
    emit_bulk_result(&updated, &missing, json);
    Ok(())
}

fn handle_bulk_dep_add(
    backlog_dir: &Path,
    tasks: &[Task],
    task_ids: Vec<String>,
    dependency: String,
    touch: bool,
    json: bool,
    auto_checkpoint: bool,
) -> Result<()> {
    let ids = normalize_task_ids(split_list(&task_ids));
    if ids.is_empty() {
        die("No tasks provided");
    }
    let (selected, missing) = select_tasks_with_missing(tasks, &ids);
    let mut updated = Vec::new();
    for task in selected {
        let path = task.file_path.as_ref().unwrap_or_else(|| {
            die(&format!("Task not found: {}", task.id));
        });
        let mut current = task.dependencies.clone();
        if !current.contains(&dependency) {
            current.push(dependency.clone());
        }
        set_list_field(path, "dependencies", current)?;
        if touch {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
        }
        audit_event(
            backlog_dir,
            "bulk_dependency_add",
            Some(&task.id),
            serde_json::json!({ "dependency": dependency.clone() }),
        )?;
        updated.push(task.id.clone());
    }
    refresh_index_best_effort(backlog_dir);
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint);
    emit_bulk_result(&updated, &missing, json);
    Ok(())
}

fn handle_bulk_dep_remove(
    backlog_dir: &Path,
    tasks: &[Task],
    task_ids: Vec<String>,
    dependency: String,
    touch: bool,
    json: bool,
    auto_checkpoint: bool,
) -> Result<()> {
    let ids = normalize_task_ids(split_list(&task_ids));
    if ids.is_empty() {
        die("No tasks provided");
    }
    let (selected, missing) = select_tasks_with_missing(tasks, &ids);
    let mut updated = Vec::new();
    for task in selected {
        let path = task.file_path.as_ref().unwrap_or_else(|| {
            die(&format!("Task not found: {}", task.id));
        });
        let mut current = task.dependencies.clone();
        current.retain(|entry| entry != &dependency);
        set_list_field(path, "dependencies", current)?;
        if touch {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
        }
        audit_event(
            backlog_dir,
            "bulk_dependency_remove",
            Some(&task.id),
            serde_json::json!({ "dependency": dependency.clone() }),
        )?;
        updated.push(task.id.clone());
    }
    refresh_index_best_effort(backlog_dir);
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint);
    emit_bulk_result(&updated, &missing, json);
    Ok(())
}

fn handle_bulk_note(
    backlog_dir: &Path,
    tasks: &[Task],
    task_ids: Vec<String>,
    note: String,
    section: NoteSection,
    touch: bool,
    json: bool,
    auto_checkpoint: bool,
) -> Result<()> {
    let ids = normalize_task_ids(split_list(&task_ids));
    if ids.is_empty() {
        die("No tasks provided");
    }
    let (selected, missing) = select_tasks_with_missing(tasks, &ids);
    let mut updated = Vec::new();
    for task in selected {
        let path = task.file_path.as_ref().unwrap_or_else(|| {
            die(&format!("Task not found: {}", task.id));
        });
        let new_body = append_note(&task.body, &note, section.as_str());
        update_body(path, &new_body)?;
        if touch {
            update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
        }
        audit_event(
            backlog_dir,
            "bulk_note",
            Some(&task.id),
            serde_json::json!({ "section": section.as_str(), "note": note }),
        )?;
        updated.push(task.id.clone());
    }
    refresh_index_best_effort(backlog_dir);
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint);
    emit_bulk_result(&updated, &missing, json);
    Ok(())
}

fn best_practices_text() -> &'static str {
    "workmesh best practices\n\nDependencies:\n- Add dependencies whenever a task is blocked by other work.\n- Prefer explicit task ids (task-042) over vague references.\n- Update dependencies as status changes to avoid stale blockers.\n- Use validate to catch missing or broken dependency chains.\n\nLabels:\n- Use labels to group work (docs, infra, ops).\n- Keep labels short and consistent.\n\nNotes:\n- Capture blockers or decisions in notes for future context.\n"
}

fn next_id(tasks: &[Task]) -> String {
    let mut max_num = 0;
    for task in tasks {
        max_num = max_num.max(task.id_num());
    }
    format!("task-{:03}", max_num + 1)
}

fn update_list_field(
    backlog_dir: &Path,
    tasks: &[Task],
    task_id: &str,
    field: &str,
    value: &str,
    add: bool,
    touch: bool,
) -> Result<()> {
    let task = find_task(tasks, task_id).unwrap_or_else(|| {
        die(&format!("Task not found: {}", task_id));
    });
    let path = task.file_path.as_ref().unwrap_or_else(|| {
        die(&format!("Task not found: {}", task_id));
    });
    let mut current = match field {
        "labels" => task.labels.clone(),
        "dependencies" => task.dependencies.clone(),
        _ => Vec::new(),
    };
    let value = value.trim();
    if add {
        if !current.contains(&value.to_string()) {
            current.push(value.to_string());
        }
    } else {
        current.retain(|entry| entry != value);
    }
    set_list_field(path, field, current)?;
    if touch {
        update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
    }
    let action = match (field, add) {
        ("labels", true) => "label_add",
        ("labels", false) => "label_remove",
        ("dependencies", true) => "dependency_add",
        ("dependencies", false) => "dependency_remove",
        _ => "update_list",
    };
    audit_event(
        backlog_dir,
        action,
        Some(&task.id),
        serde_json::json!({ "field": field, "value": value, "add": add }),
    )?;
    refresh_index_best_effort(backlog_dir);
    let action = if add { "Added" } else { "Removed" };
    println!("{} {} on {} {}", action, value, task.id, field);
    Ok(())
}

fn read_content(text: Option<&str>, file_path: Option<&Path>) -> Result<String> {
    if let Some(path) = file_path {
        return Ok(std::fs::read_to_string(path)?);
    }
    if let Some(text) = text {
        return Ok(text.to_string());
    }
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer)
}

fn audit_event(
    backlog_dir: &Path,
    action: &str,
    task_id: Option<&str>,
    details: serde_json::Value,
) -> Result<()> {
    let actor = std::env::var("USER").ok();
    let event = AuditEvent {
        timestamp: now_timestamp(),
        actor,
        action: action.to_string(),
        task_id: task_id.map(|value| value.to_string()),
        details,
    };
    append_audit_event(backlog_dir, &event)?;
    Ok(())
}

fn auto_checkpoint_enabled(cli: &Cli) -> bool {
    if cli.auto_checkpoint {
        return true;
    }
    env_flag_true("WORKMESH_AUTO_CHECKPOINT")
}

fn env_flag_true(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_lowercase())
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn maybe_auto_checkpoint(backlog_dir: &Path, enabled: bool) {
    if !enabled {
        return;
    }
    let tasks = load_tasks(backlog_dir);
    let options = CheckpointOptions {
        project_id: None,
        checkpoint_id: None,
        audit_limit: 10,
    };
    let _ = write_checkpoint(backlog_dir, &tasks, &options);
}

fn refresh_index_best_effort(backlog_dir: &Path) {
    let _ = refresh_index(backlog_dir);
}

fn die(message: &str) -> ! {
    eprintln!("{}", message);
    std::process::exit(1);
}
