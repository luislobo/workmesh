use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand, ValueEnum};

use workmesh_core::backlog::resolve_backlog_dir;
use workmesh_core::audit::{append_audit_event, AuditEvent};
use workmesh_core::gantt::{plantuml_gantt, render_plantuml_svg, write_text_file, PlantumlRenderError};
use workmesh_core::index::{rebuild_index, refresh_index, verify_index};
use workmesh_core::quickstart::quickstart;
use workmesh_core::task::{load_tasks, Lease, Task};
use workmesh_core::project::{ensure_project_docs, repo_root_from_backlog};
use workmesh_core::task_ops::{
    append_note, create_task_file, filter_tasks, graph_export, next_task, now_timestamp,
    ready_tasks, render_task_line, replace_section, set_list_field, sort_tasks, status_counts,
    task_to_json_value, tasks_to_json, timestamp_plus_minutes, update_body, update_lease_fields,
    update_task_field, update_task_field_or_section, validate_tasks,
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
        let result = quickstart(
            &repo_root,
            project_id,
            name.as_deref(),
            *agents_snippet,
        )?;
        if *json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("Docs: {}", result.project_dir.display());
            println!("Backlog: {}", result.backlog_dir.display());
            if let Some(task_path) = result.created_task.as_ref() {
                println!("Seed task: {}", task_path.display());
            }
            if result.agents_snippet_written {
                println!("AGENTS.md updated");
            }
        }
        return Ok(());
    }
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
            let stats = status_counts(&tasks);
            if json {
                let mut map = serde_json::Map::new();
                for (key, value) in stats {
                    map.insert(key, serde_json::Value::from(value as u64));
                }
                println!("{}", serde_json::to_string_pretty(&serde_json::Value::Object(map))?);
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
        Command::SetStatus { task_id, status, touch } => {
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
            println!("Released {} lease", task.id);
        }
        Command::SetField { task_id, field, value, touch } => {
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
            println!("Updated {} {} -> {}", task.id, field, value);
        }
        Command::LabelAdd { task_id, label, touch } => {
            update_list_field(&backlog_dir, &tasks, &task_id, "labels", &label, true, touch)?;
        }
        Command::LabelRemove { task_id, label, touch } => {
            update_list_field(&backlog_dir, &tasks, &task_id, "labels", &label, false, touch)?;
        }
        Command::DepAdd { task_id, dependency, touch } => {
            update_list_field(
                &backlog_dir,
                &tasks,
                &task_id,
                "dependencies",
                &dependency,
                true,
                touch,
            )?;
        }
        Command::DepRemove { task_id, dependency, touch } => {
            update_list_field(
                &backlog_dir,
                &tasks,
                &task_id,
                "dependencies",
                &dependency,
                false,
                touch,
            )?;
        }
        Command::Note { task_id, note, section, touch } => {
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
            println!("Added note to {}", task.id);
        }
        Command::SetBody { task_id, text, file, touch } => {
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
            println!("Updated body for {}", task.id);
        }
        Command::SetSection { task_id, section, text, file, touch } => {
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
        Command::GanttFile { start, zoom, output } => {
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
            let svg = render_plantuml_svg(
                &text,
                cmd,
                plantuml_jar.as_deref(),
                None,
            )
            .map_err(|err| match err {
                PlantumlRenderError::RenderFailed(msg) => anyhow::Error::msg(msg),
                other => anyhow::Error::msg(other.to_string()),
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

fn refresh_index_best_effort(backlog_dir: &Path) {
    let _ = refresh_index(backlog_dir);
}

fn die(message: &str) -> ! {
    eprintln!("{}", message);
    std::process::exit(1);
}
