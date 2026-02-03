use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;
use regex::Regex;
use serde::Serialize;

use crate::task::{split_front_matter, Task, TaskParseError};

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    Scalar(String),
    List(Vec<String>),
}

impl FieldValue {
    pub fn as_formatted(&self) -> String {
        match self {
            FieldValue::Scalar(value) => value.to_string(),
            FieldValue::List(values) => format!("[{}]", values.join(", ")),
        }
    }
}

impl From<&str> for FieldValue {
    fn from(value: &str) -> Self {
        FieldValue::Scalar(value.to_string())
    }
}

impl From<String> for FieldValue {
    fn from(value: String) -> Self {
        FieldValue::Scalar(value)
    }
}

impl From<Vec<String>> for FieldValue {
    fn from(value: Vec<String>) -> Self {
        FieldValue::List(value)
    }
}

pub fn is_done(task: &Task) -> bool {
    task.status.trim().eq_ignore_ascii_case("done")
}

pub fn deps_satisfied(task: &Task, done_ids: &HashSet<String>) -> bool {
    task.dependencies
        .iter()
        .all(|dep| done_ids.contains(&dep.to_lowercase()))
}

pub fn filter_tasks<'a>(
    tasks: &'a [Task],
    status: Option<&[String]>,
    phase: Option<&[String]>,
    priority: Option<&[String]>,
    labels: Option<&[String]>,
    depends_on: Option<&str>,
    deps_ready: Option<bool>,
    blocked: Option<bool>,
    search: Option<&str>,
) -> Vec<&'a Task> {
    let mut result: Vec<&Task> = tasks.iter().collect();
    let done_ids: HashSet<String> = tasks
        .iter()
        .filter(|task| is_done(task))
        .map(|task| task.id.to_lowercase())
        .collect();

    if let Some(status) = status {
        let status_set: HashSet<String> = status.iter().map(|s| s.to_lowercase()).collect();
        result.retain(|task| status_set.contains(&task.status.to_lowercase()));
    }
    if let Some(phase) = phase {
        let phase_set: HashSet<String> = phase.iter().map(|p| p.to_lowercase()).collect();
        result.retain(|task| phase_set.contains(&task.phase.to_lowercase()));
    }
    if let Some(priority) = priority {
        let priority_set: HashSet<String> = priority.iter().map(|p| p.to_lowercase()).collect();
        result.retain(|task| priority_set.contains(&task.priority.to_lowercase()));
    }
    if let Some(labels) = labels {
        let label_set: HashSet<String> = labels.iter().map(|l| l.to_lowercase()).collect();
        result.retain(|task| {
            let task_labels: HashSet<String> =
                task.labels.iter().map(|l| l.to_lowercase()).collect();
            !label_set.is_disjoint(&task_labels)
        });
    }
    if let Some(depends_on) = depends_on {
        let needle = depends_on.to_lowercase();
        result.retain(|task| {
            task.dependencies
                .iter()
                .any(|dep| dep.to_lowercase() == needle)
        });
    }
    if let Some(search) = search {
        let needle = search.to_lowercase();
        result.retain(|task| {
            task.title.to_lowercase().contains(&needle)
                || task.body.to_lowercase().contains(&needle)
        });
    }
    if deps_ready.is_some() || blocked.is_some() {
        if deps_ready == Some(true) {
            result.retain(|task| deps_satisfied(task, &done_ids));
        }
        if blocked == Some(true) {
            result.retain(|task| !deps_satisfied(task, &done_ids));
        }
    }

    result
}

pub fn sort_tasks<'a>(mut tasks: Vec<&'a Task>, key: &str) -> Vec<&'a Task> {
    match key {
        "id" => tasks.sort_by_key(|task| task.id_num()),
        "title" => tasks.sort_by_key(|task| task.title.to_lowercase()),
        "status" => tasks.sort_by_key(|task| task.status.to_lowercase()),
        "phase" => tasks.sort_by_key(|task| task.phase.to_lowercase()),
        "priority" => tasks.sort_by_key(|task| task.priority.to_lowercase()),
        _ => {}
    }
    tasks
}

pub fn render_task_line(task: &Task) -> String {
    let title = if task.title.trim().is_empty() {
        "(no title)"
    } else {
        task.title.trim()
    };
    format!(
        "{} | {} | {} | {} | {}",
        task.id, task.status, task.priority, task.phase, title
    )
}

pub fn now_timestamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M").to_string()
}

pub fn update_front_matter_value(
    text: &str,
    key: &str,
    value: Option<FieldValue>,
) -> Result<String, TaskParseError> {
    if !text.starts_with("---") {
        return Err(TaskParseError::MissingFrontMatter);
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut end_idx = None;
    for (idx, line) in lines.iter().enumerate().skip(1) {
        if line.trim() == "---" {
            end_idx = Some(idx);
            break;
        }
    }
    let end_idx = end_idx.ok_or(TaskParseError::MissingFrontMatterEnd)?;
    let mut fm_lines: Vec<String> = lines[1..end_idx].iter().map(|line| (*line).to_string()).collect();

    let mut key_idx = None;
    for (idx, line) in fm_lines.iter().enumerate() {
        if is_key_line(line, key) {
            key_idx = Some(idx);
            break;
        }
    }

    if let Some(idx) = key_idx {
        let mut j = idx + 1;
        while j < fm_lines.len() {
            let next = &fm_lines[j];
            if next.starts_with(' ') || next.starts_with('\t') {
                j += 1;
                continue;
            }
            break;
        }
        fm_lines.drain(idx..j);
    }

    if let Some(value) = value {
        let insert_at = key_idx.unwrap_or_else(|| fm_lines.len());
        fm_lines.insert(insert_at, format!("{}: {}", key, value.as_formatted()));
    }

    let mut new_lines: Vec<String> = Vec::new();
    new_lines.push("---".to_string());
    new_lines.extend(fm_lines);
    new_lines.push("---".to_string());
    new_lines.extend(lines[end_idx + 1..].iter().map(|line| (*line).to_string()));

    let mut rendered = new_lines.join("\n");
    if text.ends_with('\n') {
        rendered.push('\n');
    }
    Ok(rendered)
}

pub fn update_task_field(path: &Path, key: &str, value: Option<FieldValue>) -> Result<(), TaskParseError> {
    let text = fs::read_to_string(path)
        .map_err(|err| TaskParseError::Invalid(err.to_string()))?;
    let updated = update_front_matter_value(&text, key, value)?;
    fs::write(path, updated).map_err(|err| TaskParseError::Invalid(err.to_string()))?;
    Ok(())
}

pub fn set_list_field(path: &Path, key: &str, new_list: Vec<String>) -> Result<(), TaskParseError> {
    update_task_field(path, key, Some(FieldValue::List(new_list)))
}

pub fn update_task_field_or_section(
    path: &Path,
    key: &str,
    value: Option<&str>,
) -> Result<(), TaskParseError> {
    if let Some(section) = section_name_for_field(key) {
        let text = fs::read_to_string(path)
            .map_err(|err| TaskParseError::Invalid(err.to_string()))?;
        let (front, body) = split_front_matter(&text)?;
        let content = value.unwrap_or("");
        let new_body = replace_section(&body, &section, content);
        let updated = format!("---\n{}\n---\n{}", front, new_body);
        fs::write(path, updated).map_err(|err| TaskParseError::Invalid(err.to_string()))?;
        return Ok(());
    }
    let field_value = value.map(|val| FieldValue::Scalar(val.to_string()));
    update_task_field(path, key, field_value)
}

pub fn append_note(body: &str, note: &str, section: &str) -> String {
    let mut lines: Vec<String> = body.lines().map(|line| line.to_string()).collect();
    let note_line = format!("- {}", note.trim());

    if section == "notes" {
        let idx = lines
            .iter()
            .position(|line| line.trim() == "Notes:");
        match idx {
            Some(idx) => {
                let insert_at = idx + 1;
                lines.insert(insert_at, note_line);
                return finalize_lines(lines);
            }
            None => {
                if let Some(last) = lines.last() {
                    if !last.trim().is_empty() {
                        lines.push(String::new());
                    }
                }
                lines.push("Notes:".to_string());
                lines.push(note_line);
                return finalize_lines(lines);
            }
        }
    }

    let begin = "<!-- SECTION:NOTES:BEGIN -->";
    let end = "<!-- SECTION:NOTES:END -->";
    let begin_idx = lines.iter().position(|line| line.trim() == begin);
    let end_idx = lines.iter().position(|line| line.trim() == end);

    if let (Some(begin_idx), Some(end_idx)) = (begin_idx, end_idx) {
        if begin_idx < end_idx {
            lines.insert(end_idx, note.to_string());
            return finalize_lines(lines);
        }
    }

    if let Some(last) = lines.last() {
        if !last.trim().is_empty() {
            lines.push(String::new());
        }
    }
    lines.push("## Implementation Notes".to_string());
    lines.push(String::new());
    lines.push(begin.to_string());
    lines.push(note.to_string());
    lines.push(end.to_string());
    finalize_lines(lines)
}

pub fn replace_section(body: &str, section: &str, content: &str) -> String {
    let section = section.trim();
    if section.is_empty() {
        return body.to_string();
    }
    if section.eq_ignore_ascii_case("implementation notes") || section.eq_ignore_ascii_case("impl") {
        return replace_impl_notes(body, content);
    }

    let mut lines: Vec<String> = body.lines().map(|line| line.to_string()).collect();
    let header = format!("{}:", section).to_lowercase();
    let header_idx = lines
        .iter()
        .position(|line| line.trim().to_lowercase() == header);

    if header_idx.is_none() {
        if let Some(last) = lines.last() {
            if !last.trim().is_empty() {
                lines.push(String::new());
            }
        }
        lines.push(format!("{}:", section));
        lines.push("--------------------------------------------------".to_string());
        lines.extend(normalize_section_content(content));
        return finalize_lines(lines);
    }

    let header_idx = header_idx.unwrap();
    let mut start_idx = header_idx + 1;
    if start_idx < lines.len() && is_dash_line(&lines[start_idx]) {
        start_idx += 1;
    }

    let mut end_idx = lines.len();
    for idx in start_idx..lines.len() {
        if is_section_header(&lines, idx) {
            end_idx = idx;
            break;
        }
    }

    let mut new_lines = Vec::new();
    new_lines.extend_from_slice(&lines[..start_idx]);
    new_lines.extend(normalize_section_content(content));
    new_lines.extend_from_slice(&lines[end_idx..]);
    finalize_lines(new_lines)
}

pub fn update_body(path: &Path, new_body: &str) -> Result<(), TaskParseError> {
    let text = fs::read_to_string(path)
        .map_err(|err| TaskParseError::Invalid(err.to_string()))?;
    let (front, _body) = split_front_matter(&text)?;
    let updated = format!("---\n{}\n---\n{}", front, new_body);
    fs::write(path, updated).map_err(|err| TaskParseError::Invalid(err.to_string()))?;
    Ok(())
}

pub fn create_task_file(
    tasks_dir: &Path,
    task_id: &str,
    title: &str,
    status: &str,
    priority: &str,
    phase: &str,
    dependencies: &[String],
    labels: &[String],
    assignee: &[String],
) -> Result<PathBuf, TaskParseError> {
    let filename_title = slug_title(title);
    let filename = format!("{} - {}.md", task_id, filename_title);
    let path = tasks_dir.join(filename);
    let content = task_template(
        task_id,
        title,
        status,
        priority,
        phase,
        dependencies,
        labels,
        assignee,
    );
    fs::write(&path, content).map_err(|err| TaskParseError::Invalid(err.to_string()))?;
    Ok(path)
}

pub fn next_task(tasks: &[Task]) -> Option<Task> {
    let done_ids: HashSet<String> = tasks
        .iter()
        .filter(|task| is_done(task))
        .map(|task| task.id.to_lowercase())
        .collect();
    let mut ready: Vec<&Task> = tasks
        .iter()
        .filter(|task| task.status.eq_ignore_ascii_case("to do"))
        .filter(|task| deps_satisfied(task, &done_ids))
        .collect();
    if ready.is_empty() {
        return None;
    }
    ready.sort_by_key(|task| task.id_num());
    ready.first().map(|task| (*task).clone())
}

pub fn validate_tasks(tasks: &[Task]) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let ids: Vec<String> = tasks
        .iter()
        .filter(|task| !task.id.is_empty())
        .map(|task| task.id.to_lowercase())
        .collect();
    let mut duplicates = HashSet::new();
    for id in &ids {
        if ids.iter().filter(|other| *other == id).count() > 1 {
            duplicates.insert(id.clone());
        }
    }
    let mut dup_list: Vec<String> = duplicates.into_iter().collect();
    dup_list.sort();
    for dup in dup_list {
        errors.push(format!("Duplicate task id: {}", dup));
    }

    let existing_ids: HashSet<String> = tasks.iter().map(|task| task.id.to_lowercase()).collect();
    for task in tasks {
        let mut missing = Vec::new();
        if task.id.is_empty() {
            missing.push("id");
        }
        if task.title.is_empty() {
            missing.push("title");
        }
        if task.status.is_empty() {
            missing.push("status");
        }
        if task.priority.is_empty() {
            missing.push("priority");
        }
        if task.phase.is_empty() {
            missing.push("phase");
        }
        if task.labels.is_empty() {
            missing.push("labels");
        }
        if !missing.is_empty() {
            errors.push(format!("{} missing fields: {}", task.id, missing.join(", ")));
        }
        if let Some(path) = &task.file_path {
            if !path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase()
                .contains(&task.id.to_lowercase())
            {
                warnings.push(format!(
                    "{} does not match filename {}",
                    task.id,
                    path.file_name().and_then(|s| s.to_str()).unwrap_or("")
                ));
            }
        }
        for dep in &task.dependencies {
            if !existing_ids.contains(&dep.to_lowercase()) {
                errors.push(format!("{} depends on missing task {}", task.id, dep));
            }
        }
        if should_warn_missing_dependencies(task) {
            warnings.push(format!(
                "{} has no dependencies listed; add if it depends on other tasks",
                task.id
            ));
        }
    }

    ValidationResult { errors, warnings }
}

pub fn status_counts(tasks: &[Task]) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for task in tasks {
        let key = if task.status.is_empty() {
            "(none)".to_string()
        } else {
            task.status.clone()
        };
        if let Some((_, count)) = counts.iter_mut().find(|(name, _)| *name == key) {
            *count += 1;
        } else {
            counts.push((key, 1));
        }
    }
    counts
}

pub fn tasks_to_json(tasks: &[Task], include_body: bool) -> String {
    let payload: Vec<serde_json::Value> = tasks
        .iter()
        .map(|task| task_to_json_value(task, include_body))
        .collect();
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "[]".to_string())
}

pub fn task_to_json_value(task: &Task, include_body: bool) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("id".to_string(), serde_json::Value::String(task.id.clone()));
    map.insert("title".to_string(), serde_json::Value::String(task.title.clone()));
    map.insert(
        "status".to_string(),
        serde_json::Value::String(task.status.clone()),
    );
    map.insert(
        "priority".to_string(),
        serde_json::Value::String(task.priority.clone()),
    );
    map.insert("phase".to_string(), serde_json::Value::String(task.phase.clone()));
    map.insert(
        "dependencies".to_string(),
        serde_json::Value::Array(
            task.dependencies
                .iter()
                .map(|dep| serde_json::Value::String(dep.clone()))
                .collect(),
        ),
    );
    map.insert(
        "labels".to_string(),
        serde_json::Value::Array(
            task.labels
                .iter()
                .map(|label| serde_json::Value::String(label.clone()))
                .collect(),
        ),
    );
    map.insert(
        "assignee".to_string(),
        serde_json::Value::Array(
            task.assignee
                .iter()
                .map(|label| serde_json::Value::String(label.clone()))
                .collect(),
        ),
    );
    map.insert(
        "created_date".to_string(),
        task.created_date
            .clone()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    map.insert(
        "updated_date".to_string(),
        task.updated_date
            .clone()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    map.insert(
        "extra".to_string(),
        serde_json::to_value(&task.extra).unwrap_or(serde_json::Value::Object(Default::default())),
    );
    map.insert(
        "path".to_string(),
        task.file_path
            .as_ref()
            .and_then(|path| path.to_str())
            .map(|path| serde_json::Value::String(path.to_string()))
            .unwrap_or(serde_json::Value::Null),
    );
    if include_body {
        map.insert("body".to_string(), serde_json::Value::String(task.body.clone()));
    }
    serde_json::Value::Object(map)
}

fn should_warn_missing_dependencies(task: &Task) -> bool {
    if task.status.trim().eq_ignore_ascii_case("done") {
        return false;
    }
    task.dependencies.is_empty()
}

fn slug_title(title: &str) -> String {
    let re = Regex::new(r"[^a-zA-Z0-9\s\-]").expect("regex");
    let cleaned = re.replace_all(title, "");
    let cleaned = cleaned.trim().to_lowercase();
    let cleaned = Regex::new(r"\s+")
        .expect("regex")
        .replace_all(&cleaned, " ")
        .to_string();
    if cleaned.is_empty() {
        "untitled".to_string()
    } else {
        cleaned
    }
}

fn task_template(
    task_id: &str,
    title: &str,
    status: &str,
    priority: &str,
    phase: &str,
    dependencies: &[String],
    labels: &[String],
    assignee: &[String],
) -> String {
    let mut front = Vec::new();
    front.push("---".to_string());
    front.push(format!("id: {}", task_id));
    front.push(format!("title: {}", title));
    front.push(format!("status: {}", status));
    front.push(format!("priority: {}", priority));
    front.push(format!("phase: {}", phase));
    front.push(format!("dependencies: {}", FieldValue::List(dependencies.to_vec()).as_formatted()));
    front.push(format!("labels: {}", FieldValue::List(labels.to_vec()).as_formatted()));
    front.push(format!("assignee: {}", FieldValue::List(assignee.to_vec()).as_formatted()));
    front.push("---".to_string());
    front.push(String::new());
    front.push("Description:".to_string());
    front.push("--------------------------------------------------".to_string());
    front.push("- ".to_string());
    front.push(String::new());
    front.push("Acceptance Criteria:".to_string());
    front.push("--------------------------------------------------".to_string());
    front.push("- ".to_string());
    front.push(String::new());
    front.push("Definition of Done:".to_string());
    front.push("--------------------------------------------------".to_string());
    front.push("- Code/config committed.".to_string());
    front.push("- Docs updated if needed.".to_string());
    front.push(String::new());
    front.join("\n")
}

fn normalize_section_content(content: &str) -> Vec<String> {
    let trimmed = content.trim_end_matches('\n');
    if trimmed.is_empty() {
        return Vec::new();
    }
    trimmed.lines().map(|line| line.to_string()).collect()
}

fn is_dash_line(line: &str) -> bool {
    let stripped = line.trim();
    !stripped.is_empty() && stripped.chars().all(|c| c == '-') && stripped.len() >= 3
}

fn next_non_empty(lines: &[String], start: usize) -> Option<usize> {
    for idx in start..lines.len() {
        if !lines[idx].trim().is_empty() {
            return Some(idx);
        }
    }
    None
}

fn is_section_header(lines: &[String], idx: usize) -> bool {
    let stripped = lines[idx].trim();
    if stripped.is_empty() {
        return false;
    }
    if stripped.to_lowercase().starts_with("## ") {
        return true;
    }
    if stripped.ends_with(':') {
        let known = [
            "description:",
            "acceptance criteria:",
            "definition of done:",
            "notes:",
        ];
        if known.iter().any(|value| value.eq_ignore_ascii_case(stripped)) {
            return true;
        }
        if let Some(next_idx) = next_non_empty(lines, idx + 1) {
            if is_dash_line(&lines[next_idx]) {
                return true;
            }
        }
    }
    false
}

fn replace_impl_notes(body: &str, content: &str) -> String {
    let mut lines: Vec<String> = body.lines().map(|line| line.to_string()).collect();
    let begin = "<!-- SECTION:NOTES:BEGIN -->";
    let end = "<!-- SECTION:NOTES:END -->";
    let begin_idx = lines.iter().position(|line| line.trim() == begin);
    let end_idx = lines.iter().position(|line| line.trim() == end);

    if let (Some(begin_idx), Some(end_idx)) = (begin_idx, end_idx) {
        if begin_idx < end_idx {
            let mut new_lines = Vec::new();
            new_lines.extend_from_slice(&lines[..=begin_idx]);
            new_lines.extend(normalize_section_content(content));
            new_lines.extend_from_slice(&lines[end_idx..]);
            return finalize_lines(new_lines);
        }
    }

    if let Some(last) = lines.last() {
        if !last.trim().is_empty() {
            lines.push(String::new());
        }
    }
    lines.push("## Implementation Notes".to_string());
    lines.push(String::new());
    lines.push(begin.to_string());
    lines.extend(normalize_section_content(content));
    lines.push(end.to_string());
    finalize_lines(lines)
}

fn section_name_for_field(field: &str) -> Option<String> {
    let normalized = field
        .trim()
        .to_lowercase()
        .replace('-', "_")
        .replace(' ', "_");
    let map: HashMap<&str, &str> = HashMap::from([
        ("description", "Description"),
        ("acceptance_criteria", "Acceptance Criteria"),
        ("definition_of_done", "Definition of Done"),
        ("notes", "Notes"),
        ("implementation_notes", "Implementation Notes"),
        ("impl", "Implementation Notes"),
    ]);
    map.get(normalized.as_str()).map(|value| value.to_string())
}

fn finalize_lines(lines: Vec<String>) -> String {
    let mut result = lines.join("\n");
    result = result.trim_matches('\n').to_string();
    result.push('\n');
    result
}

fn is_key_line(line: &str, key: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with(key) {
        return false;
    }
    let rest = &trimmed[key.len()..];
    if rest.is_empty() {
        return false;
    }
    if rest.starts_with(':') {
        return true;
    }
    if rest.starts_with(char::is_whitespace) {
        return rest.trim_start().starts_with(':');
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn render_task_line_uses_placeholder() {
        let task = Task {
            id: "task-001".to_string(),
            title: "".to_string(),
            status: "To Do".to_string(),
            priority: "P1".to_string(),
            phase: "Phase1".to_string(),
            dependencies: Vec::new(),
            labels: Vec::new(),
            assignee: Vec::new(),
            created_date: None,
            updated_date: None,
            extra: HashMap::new(),
            file_path: None,
            body: String::new(),
        };
        assert!(render_task_line(&task).contains("(no title)"));
    }

    #[test]
    fn update_front_matter_value_replaces_field() {
        let text = "---\nstatus: To Do\npriority: P2\n---\nBody\n";
        let updated = update_front_matter_value(
            text,
            "status",
            Some(FieldValue::Scalar("Done".to_string())),
        )
        .expect("update");
        assert!(updated.contains("status: Done"));
        assert!(!updated.contains("status: To Do"));
    }

    #[test]
    fn replace_section_adds_when_missing() {
        let body = "Description:\n--------------------------------------------------\n- Old\n";
        let updated = replace_section(body, "Notes", "- Added");
        assert!(updated.contains("Notes:"));
        assert!(updated.contains("- Added"));
    }

    #[test]
    fn append_note_inserts_notes_section() {
        let updated = append_note("", "Test note", "notes");
        assert!(updated.contains("Notes:"));
        assert!(updated.contains("- Test note"));
    }

    #[test]
    fn create_task_file_writes_template() {
        let temp = TempDir::new().expect("tempdir");
        let tasks_dir = temp.path().join("tasks");
        fs::create_dir_all(&tasks_dir).expect("tasks dir");
        let path = create_task_file(
            &tasks_dir,
            "task-001",
            "Example",
            "To Do",
            "P2",
            "Phase1",
            &[],
            &[],
            &[],
        )
        .expect("create");
        let content = fs::read_to_string(path).expect("read");
        assert!(content.contains("id: task-001"));
        assert!(content.contains("Description:"));
    }

    #[test]
    fn status_counts_preserves_first_seen_order() {
        let tasks = vec![
            Task {
                id: "task-001".to_string(),
                title: "One".to_string(),
                status: "To Do".to_string(),
                priority: "P2".to_string(),
                phase: "Phase1".to_string(),
                dependencies: Vec::new(),
                labels: Vec::new(),
                assignee: Vec::new(),
                created_date: None,
                updated_date: None,
                extra: HashMap::new(),
                file_path: None,
                body: String::new(),
            },
            Task {
                id: "task-002".to_string(),
                title: "Two".to_string(),
                status: "In Progress".to_string(),
                priority: "P2".to_string(),
                phase: "Phase1".to_string(),
                dependencies: Vec::new(),
                labels: Vec::new(),
                assignee: Vec::new(),
                created_date: None,
                updated_date: None,
                extra: HashMap::new(),
                file_path: None,
                body: String::new(),
            },
            Task {
                id: "task-003".to_string(),
                title: "Three".to_string(),
                status: "To Do".to_string(),
                priority: "P2".to_string(),
                phase: "Phase1".to_string(),
                dependencies: Vec::new(),
                labels: Vec::new(),
                assignee: Vec::new(),
                created_date: None,
                updated_date: None,
                extra: HashMap::new(),
                file_path: None,
                body: String::new(),
            },
        ];

        let counts = status_counts(&tasks);
        assert_eq!(counts[0].0, "To Do");
        assert_eq!(counts[0].1, 2);
        assert_eq!(counts[1].0, "In Progress");
        assert_eq!(counts[1].1, 1);
    }
}
