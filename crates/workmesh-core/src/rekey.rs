use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::task::{split_front_matter, TaskParseError};
use crate::task_ops::graph_export;
use crate::task::{load_tasks, load_tasks_with_archive, Task};

#[derive(Debug, Clone, Default)]
pub struct RekeyPromptOptions {
    pub include_body: bool,
    pub include_archive: bool,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct RekeyApplyOptions {
    pub apply: bool,
    /// Strict mode rewrites only structured fields (dependencies + relationships + id).
    pub strict: bool,
    pub include_archive: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RekeyChange {
    pub path: PathBuf,
    pub old_id: String,
    pub new_id: String,
    pub renamed: bool,
    pub new_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RekeyReport {
    pub ok: bool,
    pub apply: bool,
    pub strict: bool,
    pub changes: Vec<RekeyChange>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RekeyRequest {
    pub mapping: HashMap<String, String>,
    #[serde(default = "default_strict")]
    pub strict: bool,
}

fn default_strict() -> bool {
    true
}

pub fn parse_rekey_request(input: &str) -> Result<RekeyRequest, TaskParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(TaskParseError::Invalid("Empty mapping input".to_string()));
    }
    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|err| TaskParseError::Invalid(format!("Invalid JSON: {}", err)))?;
    if let Some(obj) = value.as_object() {
        if obj.contains_key("mapping") {
            let req: RekeyRequest = serde_json::from_value(value)
                .map_err(|err| TaskParseError::Invalid(format!("Invalid request: {}", err)))?;
            return Ok(req);
        }
    }
    // Back-compat: allow passing the mapping object directly.
    let mapping: HashMap<String, String> = serde_json::from_value(value)
        .map_err(|err| TaskParseError::Invalid(format!("Invalid mapping: {}", err)))?;
    Ok(RekeyRequest {
        mapping,
        strict: true,
    })
}

pub fn load_tasks_for_rekey(backlog_dir: &Path, include_archive: bool) -> Vec<Task> {
    if include_archive {
        load_tasks_with_archive(backlog_dir)
    } else {
        load_tasks(backlog_dir)
    }
}

pub fn render_rekey_prompt(backlog_dir: &Path, options: RekeyPromptOptions) -> String {
    let mut tasks = load_tasks_for_rekey(backlog_dir, options.include_archive);
    tasks.sort_by_key(|t| t.id_num());
    if let Some(limit) = options.limit {
        tasks.truncate(limit);
    }
    let graph = graph_export(&tasks);

    let tasks_payload: Vec<serde_json::Value> = tasks
        .iter()
        .map(|t| {
            let body = if options.include_body {
                Some(t.body.clone())
            } else {
                None
            };
            serde_json::json!({
                "id": t.id,
                "uid": t.uid,
                "title": t.title,
                "kind": t.kind,
                "status": t.status,
                "priority": t.priority,
                "phase": t.phase,
                "project": t.project,
                "initiative": t.initiative,
                "dependencies": t.dependencies,
                "relationships": {
                    "blocked_by": t.relationships.blocked_by,
                    "parent": t.relationships.parent,
                    "child": t.relationships.child,
                    "discovered_from": t.relationships.discovered_from,
                },
                "path": t.file_path,
                "body": body,
            })
        })
        .collect();

    let data = serde_json::json!({
        "backlog_dir": backlog_dir,
        "tasks": tasks_payload,
        "graph": graph,
        "strict_mode": true,
    });

    // This prompt is intentionally explicit about reference rewrites.
    format!(
        "You are helping migrate WorkMesh task IDs.\n\n\
GOAL\n\
- Produce a JSON mapping from old task IDs to new task IDs.\n\n\
HARD RULES\n\
- Return JSON only (no markdown).\n\
- Do not invent new tasks.\n\
- Every reference must be renumbered via the mapping.\n\
- STRICT MODE: only structured fields are rewritten (no free-text editing).\n\n\
WHAT MUST BE UPDATED (by WorkMesh after you provide the mapping)\n\
- Each task's front matter `id`.\n\
- References in `dependencies`.\n\
- References in `relationships.blocked_by`, `relationships.parent`, `relationships.child`, `relationships.discovered_from`.\n\n\
OUTPUT JSON SCHEMA\n\
{{\n\
  \"mapping\": {{ \"<old_id>\": \"<new_id>\", \"...\": \"...\" }},\n\
  \"strict\": true\n\
}}\n\n\
NEW ID FORMAT\n\
- Use `task-<init>-NNN` where `<init>` is exactly 4 lowercase letters and `NNN` is 3 digits.\n\n\
DATA (JSON)\n\
{data}\n",
        data = serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string())
    )
}

fn yaml_to_string_without_doc_marker(value: &Value) -> Result<String, TaskParseError> {
    let mut raw = serde_yaml::to_string(value)
        .map_err(|err| TaskParseError::Invalid(format!("Failed to serialize YAML: {}", err)))?;
    if raw.starts_with("---\n") {
        raw = raw.trim_start_matches("---\n").to_string();
    }
    Ok(raw)
}

fn ensure_yaml_mapping(value: Value) -> Result<serde_yaml::Mapping, TaskParseError> {
    match value {
        Value::Mapping(map) => Ok(map),
        _ => Err(TaskParseError::Invalid(
            "Front matter must be a YAML mapping".to_string(),
        )),
    }
}

fn rewrite_id_refs_in_list(list: &mut Vec<Value>, mapping_lc: &HashMap<String, String>) {
    for entry in list.iter_mut() {
        let Some(s) = entry.as_str() else { continue };
        let key = s.trim().to_lowercase();
        if let Some(new_id) = mapping_lc.get(&key) {
            *entry = Value::String(new_id.clone());
        }
    }
}

fn rewrite_known_ref_fields(map: &mut serde_yaml::Mapping, mapping_lc: &HashMap<String, String>) {
    let list_keys = ["dependencies", "blocked_by", "parent", "child", "discovered_from"];

    for key in list_keys {
        let k = Value::String(key.to_string());
        if let Some(value) = map.get_mut(&k) {
            if let Value::Sequence(seq) = value {
                rewrite_id_refs_in_list(seq, mapping_lc);
            }
        }
    }

    // relationships: { blocked_by, parent, child, discovered_from }
    let rel_key = Value::String("relationships".to_string());
    if let Some(rel) = map.get_mut(&rel_key) {
        if let Value::Mapping(rel_map) = rel {
            for key in ["blocked_by", "parent", "child", "discovered_from"] {
                let k = Value::String(key.to_string());
                if let Some(value) = rel_map.get_mut(&k) {
                    if let Value::Sequence(seq) = value {
                        rewrite_id_refs_in_list(seq, mapping_lc);
                    }
                }
            }
        }
    }
}

fn rename_task_file_prefix(old_path: &Path, old_id: &str, new_id: &str) -> Result<Option<PathBuf>, TaskParseError> {
    let file_name = old_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    if !file_name.starts_with(old_id) {
        return Ok(None);
    }
    let new_file_name = format!("{}{}", new_id, &file_name[old_id.len()..]);
    let new_path = old_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(new_file_name);
    if new_path == old_path {
        return Ok(None);
    }
    if new_path.exists() {
        return Err(TaskParseError::Invalid(format!(
            "Refusing to overwrite existing file: {}",
            new_path.display()
        )));
    }
    fs::rename(old_path, &new_path).map_err(|err| TaskParseError::Invalid(err.to_string()))?;
    Ok(Some(new_path))
}

pub fn rekey_apply(
    backlog_dir: &Path,
    request: &RekeyRequest,
    options: RekeyApplyOptions,
) -> Result<RekeyReport, TaskParseError> {
    let mut tasks = load_tasks_for_rekey(backlog_dir, options.include_archive);
    tasks.sort_by_key(|t| t.id_num());

    let mut mapping_lc: HashMap<String, String> = HashMap::new();
    for (old, new_id) in &request.mapping {
        let old_key = old.trim().to_lowercase();
        if old_key.is_empty() {
            continue;
        }
        mapping_lc.insert(old_key, new_id.trim().to_string());
    }

    let existing_ids: HashSet<String> = tasks.iter().map(|t| t.id.to_lowercase()).collect();
    let mut warnings = Vec::new();

    // Validate: all old ids exist; new ids unique.
    let mut new_ids = HashSet::new();
    let mut missing = Vec::new();
    for old in mapping_lc.keys() {
        if !existing_ids.contains(old) {
            missing.push(old.clone());
        }
    }
    if !missing.is_empty() {
        missing.sort();
        return Err(TaskParseError::Invalid(format!(
            "Mapping references missing task ids: {}",
            missing.join(", ")
        )));
    }
    for new_id in mapping_lc.values() {
        let key = new_id.to_lowercase();
        if !new_ids.insert(key.clone()) {
            return Err(TaskParseError::Invalid(format!(
                "Duplicate new id in mapping: {}",
                new_id
            )));
        }
    }

    // Plan changes.
    let mut changes = Vec::new();
    for task in &tasks {
        let old_id = task.id.clone();
        let key = old_id.to_lowercase();
        let Some(new_id) = mapping_lc.get(&key) else { continue };
        let path = task
            .file_path
            .clone()
            .ok_or_else(|| TaskParseError::Invalid(format!("Missing path for {}", old_id)))?;
        changes.push(RekeyChange {
            path,
            old_id,
            new_id: new_id.clone(),
            renamed: false,
            new_path: None,
        });
    }

    if !options.apply {
        return Ok(RekeyReport {
            ok: true,
            apply: false,
            strict: options.strict,
            changes,
            warnings,
        });
    }

    // Apply to every task file: update structured references; update id for mapped tasks.
    let mut applied: Vec<RekeyChange> = Vec::new();
    for task in &tasks {
        let old_id = task.id.clone();
        let path = task
            .file_path
            .clone()
            .ok_or_else(|| TaskParseError::Invalid(format!("Missing path for {}", old_id)))?;

        let text = fs::read_to_string(&path).map_err(|err| TaskParseError::Invalid(err.to_string()))?;
        let (front, body) = split_front_matter(&text)?;
        let parsed: Value = serde_yaml::from_str(&front)
            .map_err(|err| TaskParseError::Invalid(format!("Invalid YAML front matter in {}: {}", path.display(), err)))?;
        let mut map = ensure_yaml_mapping(parsed)?;

        // Rewrite structured references first.
        rewrite_known_ref_fields(&mut map, &mapping_lc);

        // Rekey the task's own id if present in mapping.
        let mut renamed = false;
        let mut new_path = None;
        if let Some(new_id) = mapping_lc.get(&old_id.to_lowercase()) {
            map.insert(Value::String("id".to_string()), Value::String(new_id.clone()));
            renamed = true;
        }

        let yaml_value = Value::Mapping(map);
        let rendered_front = yaml_to_string_without_doc_marker(&yaml_value)?;
        let updated = format!("---\n{}\n---\n{}", rendered_front.trim_end(), body);
        fs::write(&path, updated).map_err(|err| TaskParseError::Invalid(err.to_string()))?;

        // Rename file if the id changed.
        if renamed {
            let new_id = mapping_lc
                .get(&old_id.to_lowercase())
                .expect("mapped");
            new_path = rename_task_file_prefix(&path, &old_id, new_id)?;
        }

        if renamed {
            applied.push(RekeyChange {
                path: path.clone(),
                old_id: old_id.clone(),
                new_id: mapping_lc.get(&old_id.to_lowercase()).unwrap().clone(),
                renamed: true,
                new_path,
            });
        }
    }

    if applied.is_empty() && !mapping_lc.is_empty() {
        warnings.push("Mapping applied, but no tasks were rekeyed (check id casing/spacing).".to_string());
    }

    Ok(RekeyReport {
        ok: true,
        apply: true,
        strict: options.strict,
        changes: applied,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn safe_stem(title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }

    fn write_task(
        tasks_dir: &Path,
        id: &str,
        title: &str,
        deps: &[&str],
        rel_blocked_by: &[&str],
    ) -> PathBuf {
        let deps_list = deps.join(", ");
        let rel_list = rel_blocked_by.join(", ");
        let content = format!(
            "---\n\
id: {id}\n\
uid: 01TESTUID000000000000000000\n\
title: {title}\n\
kind: task\n\
status: To Do\n\
priority: P2\n\
phase: Phase1\n\
dependencies: [{deps_list}]\n\
relationships:\n\
  blocked_by: [{rel_list}]\n\
  parent: []\n\
  child: []\n\
  discovered_from: []\n\
---\n\
\n\
Body\n",
            id = id,
            title = title,
            deps_list = deps_list,
            rel_list = rel_list
        );
        let path = tasks_dir.join(format!(
            "{id} - {stem}.md",
            id = id,
            stem = safe_stem(title)
        ));
        fs::write(&path, content).expect("write");
        path
    }

    #[test]
    fn prompt_includes_graph_and_schema() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path().join("workmesh");
        let tasks_dir = backlog_dir.join("tasks");
        fs::create_dir_all(&tasks_dir).expect("tasks dir");
        write_task(&tasks_dir, "task-001", "Alpha", &[], &[]);
        write_task(&tasks_dir, "task-002", "Beta", &["task-001"], &[]);

        let prompt = render_rekey_prompt(&backlog_dir, RekeyPromptOptions::default());
        assert!(prompt.contains("\"mapping\""));
        assert!(prompt.contains("\"graph\""));
        assert!(prompt.contains("\"tasks\""));
        assert!(prompt.contains("dependencies"));
        assert!(prompt.contains("relationships"));
    }

    #[test]
    fn apply_rewrites_ids_and_structured_references() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path().join("workmesh");
        let tasks_dir = backlog_dir.join("tasks");
        fs::create_dir_all(&tasks_dir).expect("tasks dir");

        let a = write_task(&tasks_dir, "task-001", "Alpha", &[], &[]);
        let b = write_task(&tasks_dir, "task-002", "Beta", &["task-001"], &["task-001"]);

        // Load via the real parser to match production.
        let tasks = load_tasks(&backlog_dir);
        assert_eq!(tasks.len(), 2);

        let req = RekeyRequest {
            mapping: HashMap::from([("task-001".to_string(), "task-logi-001".to_string())]),
            strict: true,
        };
        let report = rekey_apply(
            &backlog_dir,
            &req,
            RekeyApplyOptions {
                apply: true,
                strict: true,
                include_archive: false,
            },
        )
        .expect("apply");
        assert!(report.ok);
        assert_eq!(report.changes.len(), 1);

        // Re-read the file that referenced the old id; it should now reference the new id.
        let beta_text = fs::read_to_string(&b).expect("read beta");
        let (beta_front, _beta_body) = split_front_matter(&beta_text).expect("split beta");
        let beta_yaml: Value = serde_yaml::from_str(&beta_front).expect("parse beta yaml");
        let beta_map = beta_yaml.as_mapping().expect("beta mapping");
        let deps = beta_map
            .get(&Value::String("dependencies".to_string()))
            .and_then(|v| v.as_sequence())
            .cloned()
            .unwrap_or_default();
        assert!(deps.iter().any(|v| v.as_str() == Some("task-logi-001")));
        let blocked_by = if let Some(rel) = beta_map
            .get(&Value::String("relationships".to_string()))
            .and_then(|v| v.as_mapping())
        {
            rel.get(&Value::String("blocked_by".to_string()))
                .and_then(|v| v.as_sequence())
                .cloned()
                .unwrap_or_default()
        } else {
            beta_map
                .get(&Value::String("blocked_by".to_string()))
                .and_then(|v| v.as_sequence())
                .cloned()
                .unwrap_or_default()
        };
        assert!(blocked_by.iter().any(|v| v.as_str() == Some("task-logi-001")));

        // Alpha file should be renamed (prefix replaced) if it started with old id.
        let renamed_path = report.changes[0].new_path.clone().expect("new path");
        let renamed_text = fs::read_to_string(&renamed_path).expect("read renamed");
        assert!(renamed_text.contains("id: task-logi-001"));
        assert!(!a.exists());
    }
}
