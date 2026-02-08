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
    false
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
        strict: false,
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
        "strict_mode": false,
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
- Default behavior rewrites structured fields AND updates free-text mentions in task bodies.\n\
- If you want structured-only rewrites, set `strict: true`.\n\n\
WHAT MUST BE UPDATED (by WorkMesh after you provide the mapping)\n\
- Each task's front matter `id`.\n\
- References in `dependencies`.\n\
- References in `relationships.blocked_by`, `relationships.parent`, `relationships.child`, `relationships.discovered_from`.\n\n\
- Free-text mentions of task IDs in task bodies (unless `strict: true`).\n\n\
OUTPUT JSON SCHEMA\n\
{{\n\
  \"mapping\": {{ \"<old_id>\": \"<new_id>\", \"...\": \"...\" }},\n\
  \"strict\": false\n\
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

fn parse_front_matter_tolerant(front: &str) -> serde_yaml::Mapping {
    // Prefer strict YAML when it works; otherwise fallback to a tolerant line parser.
    // This keeps rekey working on legacy front matter like `title: Phase 1: ...` (colon in scalar).
    if let Ok(value) = serde_yaml::from_str::<Value>(front) {
        if let Value::Mapping(map) = value {
            return map;
        }
    }
    let data = parse_front_matter_loose(front);
    let mut map = serde_yaml::Mapping::new();
    for (key, value) in data {
        map.insert(Value::String(key), value);
    }
    map
}

fn parse_front_matter_loose(front: &str) -> HashMap<String, Value> {
    // Keep this parser intentionally small and forgiving; it mirrors the behavior of the task loader.
    let mut data = HashMap::new();
    let lines: Vec<&str> = front.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }
        let Some((key, rest)) = line.split_once(':') else {
            i += 1;
            continue;
        };
        let key = key.trim().to_string();
        let value = rest.trim();
        if value == ">-" || value == "|" {
            let mut block: Vec<String> = Vec::new();
            i += 1;
            while i < lines.len() {
                let next = lines[i];
                if next.starts_with(' ') || next.starts_with('\t') {
                    block.push(next.trim().to_string());
                    i += 1;
                    continue;
                }
                break;
            }
            let joined = if value == ">-" {
                block.join(" ").trim().to_string()
            } else {
                block.join("\n")
            };
            data.insert(key, Value::String(joined));
            continue;
        }
        if value.is_empty() {
            let mut items: Vec<Value> = Vec::new();
            let mut j = i + 1;
            while j < lines.len() {
                let next = lines[j];
                if next.trim_start().starts_with("- ") {
                    let item = next.trim_start()[2..].trim();
                    if !item.is_empty() {
                        items.push(Value::String(item.to_string()));
                    }
                    j += 1;
                    continue;
                }
                break;
            }
            data.insert(key, Value::Sequence(items));
            i = j;
            continue;
        }
        if value.starts_with('[') && value.ends_with(']') {
            let inner = value.trim_matches(&['[', ']'][..]).trim();
            if inner.is_empty() {
                data.insert(key, Value::Sequence(Vec::new()));
            } else {
                let values = inner
                    .split(',')
                    .map(|entry| entry.trim().trim_matches('"').to_string())
                    .filter(|entry| !entry.is_empty())
                    .map(Value::String)
                    .collect::<Vec<_>>();
                data.insert(key, Value::Sequence(values));
            }
            i += 1;
            continue;
        }
        data.insert(key, Value::String(value.to_string()));
        i += 1;
    }
    data
}

fn rewrite_id_refs_in_list_count(list: &mut Vec<Value>, mapping_lc: &HashMap<String, String>) -> usize {
    let mut changed = 0usize;
    for entry in list.iter_mut() {
        let Some(s) = entry.as_str() else { continue };
        let key = s.trim().to_lowercase();
        if let Some(new_id) = mapping_lc.get(&key) {
            if s != new_id {
                *entry = Value::String(new_id.clone());
                changed += 1;
            }
        }
    }
    changed
}

fn rewrite_known_ref_fields(map: &mut serde_yaml::Mapping, mapping_lc: &HashMap<String, String>) -> usize {
    let list_keys = ["dependencies", "blocked_by", "parent", "child", "discovered_from"];
    let mut changed = 0usize;

    for key in list_keys {
        let k = Value::String(key.to_string());
        if let Some(value) = map.get_mut(&k) {
            if let Value::Sequence(seq) = value {
                changed += rewrite_id_refs_in_list_count(seq, mapping_lc);
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
                        changed += rewrite_id_refs_in_list_count(seq, mapping_lc);
                    }
                }
            }
        }
    }

    changed
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

fn is_id_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
}

fn rewrite_body_text(body: &str, mapping_lc: &HashMap<String, String>) -> (String, usize) {
    // This is intentionally conservative:
    // - only rewrites tokens that look like task IDs (`task-...`)
    // - only rewrites exact mapping hits
    // - only rewrites when the match is bounded by non-id characters
    //
    // Rust regex does not support look-around, so we do boundary checks manually.
    let re = regex::Regex::new(r"(?i)task-[a-z0-9-]+").expect("regex");
    let bytes = body.as_bytes();
    let mut out = String::with_capacity(body.len());
    let mut last = 0usize;
    let mut changed = 0usize;

    for m in re.find_iter(body) {
        let start = m.start();
        let end = m.end();

        let before_ok = start == 0 || !is_id_char(bytes[start.saturating_sub(1)]);
        let after_ok = end == bytes.len() || !is_id_char(bytes[end]);
        if !(before_ok && after_ok) {
            continue;
        }

        let matched = &body[start..end];
        let key = matched.to_lowercase();
        let Some(new_id) = mapping_lc.get(&key) else {
            continue;
        };

        out.push_str(&body[last..start]);
        out.push_str(new_id);
        last = end;
        changed += 1;
    }

    if changed == 0 {
        return (body.to_string(), 0);
    }

    out.push_str(&body[last..]);
    (out, changed)
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
        if options.strict {
            return Err(TaskParseError::Invalid(format!(
                "Mapping references missing task ids: {}",
                missing.join(", ")
            )));
        }
        warnings.push(format!(
            "Non-strict mode: continuing despite missing mapping ids: {}",
            missing.join(", ")
        ));
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
        let mut map = parse_front_matter_tolerant(&front);

        // Rewrite structured references first.
        let structured_changes = rewrite_known_ref_fields(&mut map, &mapping_lc);

        // Optionally rewrite free-text body references.
        let (new_body, body_changes) = if options.strict {
            (body.clone(), 0usize)
        } else {
            rewrite_body_text(&body, &mapping_lc)
        };

        // Rekey the task's own id if present in mapping.
        let mut renamed = false;
        let mut new_path = None;
        if let Some(new_id) = mapping_lc.get(&old_id.to_lowercase()) {
            map.insert(Value::String("id".to_string()), Value::String(new_id.clone()));
            renamed = true;
        }

        let needs_front_rewrite = renamed || structured_changes > 0;
        let needs_body_rewrite = body_changes > 0;
        if !(needs_front_rewrite || needs_body_rewrite) {
            continue;
        }

        let rendered_front = if needs_front_rewrite {
            let yaml_value = Value::Mapping(map);
            yaml_to_string_without_doc_marker(&yaml_value)?
        } else {
            front
        };

        let updated = format!(
            "---\n{}\n---\n{}",
            rendered_front.trim_end(),
            new_body
        );
        if updated != text {
            fs::write(&path, updated).map_err(|err| TaskParseError::Invalid(err.to_string()))?;
        }

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

        let a = write_task(&tasks_dir, "task-001", "Alpha: with colon", &[], &[]);
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

    #[test]
    fn apply_non_strict_rewrites_body_refs_even_when_ids_are_missing() {
        let temp = TempDir::new().expect("tempdir");
        let backlog_dir = temp.path().join("workmesh");
        let tasks_dir = backlog_dir.join("tasks");
        fs::create_dir_all(&tasks_dir).expect("tasks dir");

        // These tasks already use the new id format, but their bodies reference legacy ids.
        let path = tasks_dir.join("task-main-001 - alpha.md");
        let content = "---\n\
id: task-main-001\n\
uid: 01TESTUID000000000000000000\n\
title: Alpha\n\
kind: task\n\
status: To Do\n\
priority: P2\n\
phase: Phase1\n\
dependencies: []\n\
relationships:\n\
  blocked_by: []\n\
  parent: []\n\
  child: []\n\
  discovered_from: []\n\
---\n\
\n\
Body mentions task-001 and task-002.\n";
        fs::write(&path, content).expect("write");

        let req = RekeyRequest {
            mapping: HashMap::from([
                ("task-001".to_string(), "task-main-001".to_string()),
                ("task-002".to_string(), "task-main-002".to_string()),
            ]),
            strict: false,
        };
        let report = rekey_apply(
            &backlog_dir,
            &req,
            RekeyApplyOptions {
                apply: true,
                strict: false,
                include_archive: false,
            },
        )
        .expect("apply");
        assert!(report.ok);

        let updated = fs::read_to_string(&path).expect("read updated");
        assert!(updated.contains("Body mentions task-main-001 and task-main-002."));
        assert!(!updated.contains("task-001"));
        assert!(!updated.contains("task-002"));
    }

    #[test]
    fn parse_rekey_request_defaults_to_non_strict_when_missing_strict_flag() {
        let req =
            parse_rekey_request("{\"mapping\": {\"task-001\": \"task-main-001\"}}").expect("parse");
        assert!(!req.strict);
    }
}
