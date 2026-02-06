use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use chrono::{Local, NaiveDate};
use regex::Regex;
use thiserror::Error;

use crate::task::Task;

pub const DEFAULT_PHASE_ORDER: [&str; 7] = [
    "Preflight",
    "Phase1",
    "Phase2",
    "Phase3",
    "Phase4",
    "Phase5",
    "Unphased",
];

pub fn default_phase_durations() -> HashMap<&'static str, i32> {
    HashMap::from([
        ("Preflight", 1),
        ("Phase1", 2),
        ("Phase2", 3),
        ("Phase3", 3),
        ("Phase4", 2),
        ("Phase5", 1),
        ("Unphased", 1),
    ])
}

fn status_color_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("done", "green"),
        ("in progress", "blue"),
        ("blocked", "red"),
        ("to do", "white"),
    ])
}

#[derive(Debug, Error)]
pub enum PlantumlRenderError {
    #[error("PlantUML not found. Set XH_TASKS_PLANTUML_CMD or XH_TASKS_PLANTUML_JAR, or install the plantuml CLI.")]
    MissingPlantuml,
    #[error("PlantUML render failed: {0}")]
    RenderFailed(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn plantuml_gantt(
    tasks: &[Task],
    start: Option<&str>,
    phase_order: Option<&[String]>,
    zoom: i32,
    phase_durations: Option<HashMap<String, i32>>,
    include_dependencies: bool,
) -> String {
    let start_str = start_to_iso(start);
    let task_list: Vec<&Task> = tasks.iter().collect();
    let phases = group_by_phase(&task_list);
    let order = phase_order_list(phases.keys().cloned().collect(), phase_order);
    let mut durations: HashMap<String, i32> = default_phase_durations()
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect();
    if let Some(overrides) = phase_durations {
        for (key, value) in overrides {
            durations.insert(key, value);
        }
    }
    let done_ids: HashSet<String> = task_list
        .iter()
        .filter(|task| task.status.trim().eq_ignore_ascii_case("done"))
        .map(|task| task.id.to_lowercase())
        .collect();
    let color_map = status_color_map();

    let mut lines = vec![
        "@startgantt".to_string(),
        format!("Project starts {}", start_str),
        "printscale daily".to_string(),
    ];
    if zoom != 0 && zoom != 1 {
        lines.push(format!("scale {}", zoom));
    }
    lines.push(String::new());

    for phase in order {
        let items = phases.get(&phase).cloned().unwrap_or_default();
        if items.is_empty() {
            continue;
        }
        lines.push(format!("-- {} --", phase));
        let mut sorted_items = items;
        sorted_items.sort_by_key(|task| task.id_num());
        for task in sorted_items {
            let title = safe_title(task.title.as_str());
            let label = format!("{} {}", task.id, title);
            let duration = duration_for_task(task, &durations);
            let days = if duration == 1 { "day" } else { "days" };
            lines.push(format!("[{}] lasts {} {}", label, duration, days));
            let status_key = status_key(task, &done_ids);
            let color = color_map.get(status_key.as_str()).unwrap_or(&"white");
            lines.push(format!("[{}] is colored in {}", label, color));
        }
        lines.push(String::new());
    }

    if include_dependencies {
        lines.push("' Dependencies".to_string());
        let mut id_map: HashMap<String, String> = HashMap::new();
        for task in &task_list {
            if task.id.is_empty() {
                continue;
            }
            id_map.insert(
                task.id.to_lowercase(),
                format!("{} {}", task.id, safe_title(task.title.as_str())),
            );
        }
        for task in &task_list {
            for dep in &task.dependencies {
                if dep.trim().is_empty() {
                    continue;
                }
                let dep_label = id_map
                    .get(&dep.to_lowercase())
                    .cloned()
                    .unwrap_or_else(|| dep.clone());
                let task_label = id_map
                    .get(&task.id.to_lowercase())
                    .cloned()
                    .unwrap_or_else(|| task.id.clone());
                lines.push(format!("[{}] --> [{}]", dep_label, task_label));
            }
        }
    }

    lines.push("@endgantt".to_string());
    lines.join("\n") + "\n"
}

pub fn write_text_file(path: &Path, content: &str) -> Result<PathBuf, std::io::Error> {
    let path = expand_user(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, content)?;
    Ok(path)
}

pub fn render_plantuml_svg(
    source: &str,
    cmd: Option<Vec<String>>,
    jar_path: Option<&Path>,
    env_map: Option<HashMap<String, String>>,
) -> Result<String, PlantumlRenderError> {
    let command = resolve_plantuml_command(cmd, jar_path, env_map.as_ref())?;
    let mut process = Command::new(&command[0])
        .args(&command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = process.stdin.take() {
        use std::io::Write;
        stdin.write_all(source.as_bytes())?;
    }

    let output = process.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "PlantUML render failed".to_string()
        } else {
            stderr
        };
        return Err(PlantumlRenderError::RenderFailed(message));
    }
    let svg = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(strip_timegrid(&svg))
}

fn resolve_plantuml_command(
    cmd: Option<Vec<String>>,
    jar_path: Option<&Path>,
    env_map: Option<&HashMap<String, String>>,
) -> Result<Vec<String>, PlantumlRenderError> {
    if let Some(cmd) = cmd {
        return Ok(ensure_svg_pipe(cmd));
    }

    let env_map = env_map.cloned().unwrap_or_else(|| env::vars().collect());
    if let Some(env_cmd) = env_map
        .get("XH_TASKS_PLANTUML_CMD")
        .or_else(|| env_map.get("PLANTUML_CMD"))
    {
        let parts =
            shell_words::split(env_cmd).map_err(|_| PlantumlRenderError::MissingPlantuml)?;
        return Ok(ensure_svg_pipe(parts));
    }

    let jar_env = env_map
        .get("XH_TASKS_PLANTUML_JAR")
        .or_else(|| env_map.get("PLANTUML_JAR"))
        .cloned();
    if let Some(jar) = jar_path
        .map(|path| path.to_path_buf())
        .or(jar_env.map(PathBuf::from))
    {
        return Ok(ensure_svg_pipe(vec![
            "java".to_string(),
            "-jar".to_string(),
            jar.to_string_lossy().to_string(),
        ]));
    }

    if let Ok(plantuml) = which::which("plantuml") {
        return Ok(ensure_svg_pipe(vec![plantuml
            .to_string_lossy()
            .to_string()]));
    }

    Err(PlantumlRenderError::MissingPlantuml)
}

fn ensure_svg_pipe(args: Vec<String>) -> Vec<String> {
    let mut next = args;
    if !next.iter().any(|arg| arg == "-tsvg") {
        next.push("-tsvg".to_string());
    }
    if !next.iter().any(|arg| arg == "-pipe") {
        next.push("-pipe".to_string());
    }
    next
}

fn start_to_iso(start: Option<&str>) -> String {
    if let Some(start) = start {
        if let Ok(date) = NaiveDate::parse_from_str(start, "%Y-%m-%d") {
            return date.format("%Y-%m-%d").to_string();
        }
    }
    Local::now().date_naive().format("%Y-%m-%d").to_string()
}

fn group_by_phase<'a>(tasks: &[&'a Task]) -> HashMap<String, Vec<&'a Task>> {
    let mut phases: HashMap<String, Vec<&Task>> = HashMap::new();
    for task in tasks {
        let phase = task.phase.trim();
        let phase = if phase.is_empty() { "Unphased" } else { phase };
        phases.entry(phase.to_string()).or_default().push(*task);
    }
    phases
}

fn phase_order_list(phases: Vec<String>, override_order: Option<&[String]>) -> Vec<String> {
    let base: Vec<String> = override_order
        .map(|values| values.to_vec())
        .unwrap_or_else(|| DEFAULT_PHASE_ORDER.iter().map(|p| p.to_string()).collect());
    let phase_set: HashSet<String> = phases.iter().cloned().collect();
    let mut ordered: Vec<String> = base
        .iter()
        .filter(|phase| phase_set.contains(*phase))
        .cloned()
        .collect();
    let mut extras: Vec<String> = phases
        .into_iter()
        .filter(|phase| !base.contains(phase))
        .collect();
    extras.sort_by_key(|phase| phase.to_lowercase());
    ordered.extend(extras);
    ordered
}

fn safe_title(title: &str) -> String {
    if title.trim().is_empty() {
        return "(no title)".to_string();
    }
    title.replace('[', "(").replace(']', ")")
}

fn duration_for_task(task: &Task, durations: &HashMap<String, i32>) -> i32 {
    let phase = task.phase.trim();
    let phase = if phase.is_empty() { "Unphased" } else { phase };
    let base = durations.get(phase).cloned().unwrap_or(1);
    let extra = std::cmp::min(task.dependencies.len(), 2) as i32;
    std::cmp::max(1, base + extra)
}

fn status_key(task: &Task, done_ids: &HashSet<String>) -> String {
    let status = task.status.trim().to_lowercase();
    if status == "done" {
        return "done".to_string();
    }
    if status == "in progress" {
        return "in progress".to_string();
    }
    if !task.dependencies.is_empty()
        && !task
            .dependencies
            .iter()
            .all(|dep| done_ids.contains(&dep.to_lowercase()))
    {
        return "blocked".to_string();
    }
    "to do".to_string()
}

fn strip_timegrid(svg: &str) -> String {
    let line_re = Regex::new(r"<line\b[^>]*?>").expect("regex");
    let attr_re = Regex::new(r#"(\w+)="([^"]+)""#).expect("regex");

    line_re
        .replace_all(svg, |caps: &regex::Captures| {
            let tag = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            if !tag.contains("stroke: #C0C0C0") {
                return tag.to_string();
            }
            let mut x1: Option<String> = None;
            let mut x2: Option<String> = None;
            for cap in attr_re.captures_iter(tag) {
                let name = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let value = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                if name == "x1" {
                    x1 = Some(value.to_string());
                }
                if name == "x2" {
                    x2 = Some(value.to_string());
                }
            }
            if let (Some(x1), Some(x2)) = (x1, x2) {
                if x1 == x2 {
                    return String::new();
                }
            }
            tag.to_string()
        })
        .to_string()
}

fn expand_user(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    if let Some(stripped) = path_str.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }
    path.to_path_buf()
}
