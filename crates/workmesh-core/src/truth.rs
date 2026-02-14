use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::Local;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use ulid::Ulid;

use crate::global_sessions::{load_sessions_latest, resolve_workmesh_home};
use crate::task::load_tasks;

#[derive(Debug, Error)]
pub enum TruthError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Invalid truth operation: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TruthState {
    Proposed,
    Accepted,
    Rejected,
    Superseded,
}

impl TruthState {
    pub fn as_str(self) -> &'static str {
        match self {
            TruthState::Proposed => "proposed",
            TruthState::Accepted => "accepted",
            TruthState::Rejected => "rejected",
            TruthState::Superseded => "superseded",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "proposed" => Some(Self::Proposed),
            "accepted" => Some(Self::Accepted),
            "rejected" => Some(Self::Rejected),
            "superseded" => Some(Self::Superseded),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TruthContext {
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub epic_id: Option<String>,
    #[serde(default)]
    pub feature: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub worktree_id: Option<String>,
    #[serde(default)]
    pub worktree_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruthHistoryEntry {
    pub event_id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub by_truth_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruthRecord {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: u32,
    pub state: TruthState,
    pub title: String,
    pub statement: String,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub context: TruthContext,
    #[serde(default)]
    pub accepted_at: Option<String>,
    #[serde(default)]
    pub rejected_at: Option<String>,
    #[serde(default)]
    pub superseded_at: Option<String>,
    #[serde(default)]
    pub superseded_by: Option<String>,
    #[serde(default)]
    pub history: Vec<TruthHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TruthEvent {
    Proposed {
        event_id: String,
        truth_id: String,
        timestamp: String,
        #[serde(default)]
        actor: Option<String>,
        title: String,
        statement: String,
        #[serde(default)]
        rationale: Option<String>,
        #[serde(default)]
        constraints: Vec<String>,
        #[serde(default)]
        tags: Vec<String>,
        context: TruthContext,
    },
    Accepted {
        event_id: String,
        truth_id: String,
        timestamp: String,
        #[serde(default)]
        actor: Option<String>,
        #[serde(default)]
        note: Option<String>,
    },
    Rejected {
        event_id: String,
        truth_id: String,
        timestamp: String,
        #[serde(default)]
        actor: Option<String>,
        #[serde(default)]
        reason: Option<String>,
    },
    Superseded {
        event_id: String,
        truth_id: String,
        timestamp: String,
        #[serde(default)]
        actor: Option<String>,
        by_truth_id: String,
        #[serde(default)]
        reason: Option<String>,
    },
}

impl TruthEvent {
    fn event_id(&self) -> &str {
        match self {
            TruthEvent::Proposed { event_id, .. }
            | TruthEvent::Accepted { event_id, .. }
            | TruthEvent::Rejected { event_id, .. }
            | TruthEvent::Superseded { event_id, .. } => event_id,
        }
    }

    fn timestamp(&self) -> &str {
        match self {
            TruthEvent::Proposed { timestamp, .. }
            | TruthEvent::Accepted { timestamp, .. }
            | TruthEvent::Rejected { timestamp, .. }
            | TruthEvent::Superseded { timestamp, .. } => timestamp,
        }
    }

    fn actor(&self) -> Option<String> {
        match self {
            TruthEvent::Proposed { actor, .. }
            | TruthEvent::Accepted { actor, .. }
            | TruthEvent::Rejected { actor, .. }
            | TruthEvent::Superseded { actor, .. } => actor.clone(),
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            TruthEvent::Proposed { .. } => "proposed",
            TruthEvent::Accepted { .. } => "accepted",
            TruthEvent::Rejected { .. } => "rejected",
            TruthEvent::Superseded { .. } => "superseded",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TruthProposeInput {
    pub id: Option<String>,
    pub title: String,
    pub statement: String,
    pub rationale: Option<String>,
    pub constraints: Vec<String>,
    pub tags: Vec<String>,
    pub context: TruthContext,
    pub actor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TruthTransitionInput {
    pub truth_id: String,
    pub note: Option<String>,
    pub actor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TruthSupersedeInput {
    pub truth_id: String,
    pub by_truth_id: String,
    pub reason: Option<String>,
    pub actor: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TruthQuery {
    pub states: Vec<TruthState>,
    pub project_id: Option<String>,
    pub epic_id: Option<String>,
    pub feature: Option<String>,
    pub session_id: Option<String>,
    pub worktree_id: Option<String>,
    pub worktree_path: Option<String>,
    pub tags: Vec<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruthProjectionSummary {
    pub events: usize,
    pub records: usize,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruthValidationReport {
    pub ok: bool,
    pub events_path: String,
    pub current_path: String,
    pub event_count: usize,
    pub record_count: usize,
    pub malformed_events: Vec<String>,
    pub transition_errors: Vec<String>,
    pub projection_mismatches: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruthStoreStatus {
    pub events_path: String,
    pub current_path: String,
    pub has_events: bool,
    pub has_current: bool,
    pub event_count: usize,
    pub record_count: usize,
    pub validation_ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruthLegacyCandidate {
    pub source_type: String,
    pub source_id: String,
    pub source_path: String,
    pub statement: String,
    pub suggested_title: String,
    pub fingerprint: String,
    pub context: TruthContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruthMigrationAudit {
    pub generated_at: String,
    pub candidates: Vec<TruthLegacyCandidate>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruthMigrationPlanItem {
    pub candidate: TruthLegacyCandidate,
    pub action: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruthMigrationPlan {
    pub generated_at: String,
    pub to_create: Vec<TruthMigrationPlanItem>,
    pub skipped: Vec<TruthMigrationPlanItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruthMigrationResult {
    pub dry_run: bool,
    pub created_ids: Vec<String>,
    pub skipped: Vec<String>,
}

pub fn truth_dir(backlog_dir: &Path) -> PathBuf {
    backlog_dir.join("truth")
}

pub fn truth_events_path(backlog_dir: &Path) -> PathBuf {
    truth_dir(backlog_dir).join("events.jsonl")
}

pub fn truth_current_path(backlog_dir: &Path) -> PathBuf {
    truth_dir(backlog_dir).join("current.jsonl")
}

pub fn ensure_truth_dirs(backlog_dir: &Path) -> Result<(), TruthError> {
    fs::create_dir_all(truth_dir(backlog_dir))?;
    Ok(())
}

pub fn new_truth_id() -> String {
    format!("truth-{}", Ulid::new().to_string().to_lowercase())
}

pub fn propose_truth(backlog_dir: &Path, input: TruthProposeInput) -> Result<TruthRecord, TruthError> {
    ensure_truth_dirs(backlog_dir)?;

    let truth_id = input
        .id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(new_truth_id);

    let title = normalized_non_empty("title", &input.title)?;
    let statement = normalized_non_empty("statement", &input.statement)?;
    let rationale = normalize_optional(input.rationale);
    let constraints = normalize_list(input.constraints);
    let tags = normalize_list(input.tags);
    let context = normalize_context(input.context);

    let projected = project_from_events(backlog_dir)?;
    if projected.contains_key(&truth_id) {
        return Err(TruthError::Invalid(format!(
            "truth id already exists: {}",
            truth_id
        )));
    }

    let event = TruthEvent::Proposed {
        event_id: Ulid::new().to_string(),
        truth_id: truth_id.clone(),
        timestamp: now_rfc3339(),
        actor: normalize_optional(input.actor),
        title,
        statement,
        rationale,
        constraints,
        tags,
        context,
    };

    append_truth_event(backlog_dir, &event)?;
    rebuild_truth_projection(backlog_dir)?;

    show_truth(backlog_dir, &truth_id)?.ok_or_else(|| {
        TruthError::Invalid(format!(
            "truth {} not found after proposal write",
            truth_id
        ))
    })
}

pub fn accept_truth(backlog_dir: &Path, input: TruthTransitionInput) -> Result<TruthRecord, TruthError> {
    apply_transition(backlog_dir, input, true)
}

pub fn reject_truth(backlog_dir: &Path, input: TruthTransitionInput) -> Result<TruthRecord, TruthError> {
    apply_transition(backlog_dir, input, false)
}

pub fn supersede_truth(
    backlog_dir: &Path,
    input: TruthSupersedeInput,
) -> Result<TruthRecord, TruthError> {
    ensure_truth_dirs(backlog_dir)?;
    let truth_id = normalized_non_empty("truth_id", &input.truth_id)?;
    let by_truth_id = normalized_non_empty("by_truth_id", &input.by_truth_id)?;
    if truth_id.eq_ignore_ascii_case(&by_truth_id) {
        return Err(TruthError::Invalid(
            "superseded truth id must differ from replacement truth id".to_string(),
        ));
    }

    let projected = project_from_events(backlog_dir)?;
    let source = projected.get(&truth_id).ok_or_else(|| {
        TruthError::Invalid(format!("unknown truth id: {}", truth_id))
    })?;
    if source.state != TruthState::Accepted {
        return Err(TruthError::Invalid(format!(
            "truth {} is {} and cannot be superseded",
            truth_id,
            source.state.as_str()
        )));
    }
    let replacement = projected.get(&by_truth_id).ok_or_else(|| {
        TruthError::Invalid(format!("replacement truth not found: {}", by_truth_id))
    })?;
    if replacement.state != TruthState::Accepted {
        return Err(TruthError::Invalid(format!(
            "replacement truth {} must be accepted (found {})",
            by_truth_id,
            replacement.state.as_str()
        )));
    }

    let event = TruthEvent::Superseded {
        event_id: Ulid::new().to_string(),
        truth_id: truth_id.clone(),
        timestamp: now_rfc3339(),
        actor: normalize_optional(input.actor),
        by_truth_id,
        reason: normalize_optional(input.reason),
    };

    append_truth_event(backlog_dir, &event)?;
    rebuild_truth_projection(backlog_dir)?;
    show_truth(backlog_dir, &truth_id)?.ok_or_else(|| {
        TruthError::Invalid(format!(
            "truth {} not found after supersede write",
            truth_id
        ))
    })
}

pub fn show_truth(backlog_dir: &Path, truth_id: &str) -> Result<Option<TruthRecord>, TruthError> {
    let truth_id = truth_id.trim();
    if truth_id.is_empty() {
        return Err(TruthError::Invalid("truth id is required".to_string()));
    }
    let projected = project_from_events(backlog_dir)?;
    Ok(projected
        .into_values()
        .find(|record| record.id.eq_ignore_ascii_case(truth_id)))
}

pub fn list_truths(backlog_dir: &Path, query: &TruthQuery) -> Result<Vec<TruthRecord>, TruthError> {
    let projected = project_from_events(backlog_dir)?;
    let mut records: Vec<TruthRecord> = projected
        .into_values()
        .filter(|record| matches_query(record, query))
        .collect();

    records.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.id.to_lowercase().cmp(&b.id.to_lowercase()))
    });

    if let Some(limit) = query.limit {
        records.truncate(limit);
    }
    Ok(records)
}

pub fn rebuild_truth_projection(backlog_dir: &Path) -> Result<TruthProjectionSummary, TruthError> {
    ensure_truth_dirs(backlog_dir)?;
    let events = read_events_strict(backlog_dir)?;
    let projected = project_events(&events)?;
    write_current_projection(backlog_dir, projected.values())?;

    Ok(TruthProjectionSummary {
        events: events.len(),
        records: projected.len(),
        path: truth_current_path(backlog_dir).to_string_lossy().to_string(),
    })
}

pub fn validate_truth_store(backlog_dir: &Path) -> Result<TruthValidationReport, TruthError> {
    let events_path = truth_events_path(backlog_dir);
    let current_path = truth_current_path(backlog_dir);
    let (events, malformed_events) = read_events_with_errors(backlog_dir)?;

    let mut transition_errors = Vec::new();
    let mut projected = BTreeMap::<String, TruthRecord>::new();
    for event in &events {
        if let Err(err) = apply_event(&mut projected, event) {
            transition_errors.push(err);
        }
    }

    let current_records = read_current_records_with_errors(backlog_dir, &mut transition_errors)?;
    let projection_mismatches = compare_projection(&projected, &current_records);

    Ok(TruthValidationReport {
        ok: malformed_events.is_empty()
            && transition_errors.is_empty()
            && projection_mismatches.is_empty(),
        events_path: events_path.to_string_lossy().to_string(),
        current_path: current_path.to_string_lossy().to_string(),
        event_count: events.len(),
        record_count: projected.len(),
        malformed_events,
        transition_errors,
        projection_mismatches,
    })
}

pub fn truth_store_status(backlog_dir: &Path) -> Result<TruthStoreStatus, TruthError> {
    let report = validate_truth_store(backlog_dir)?;
    Ok(TruthStoreStatus {
        events_path: report.events_path.clone(),
        current_path: report.current_path.clone(),
        has_events: Path::new(&report.events_path).exists(),
        has_current: Path::new(&report.current_path).exists(),
        event_count: report.event_count,
        record_count: report.record_count,
        validation_ok: report.ok,
    })
}

pub fn truth_migration_audit(backlog_dir: &Path) -> Result<TruthMigrationAudit, TruthError> {
    let mut candidates = Vec::new();
    let mut warnings = Vec::new();

    let decision_re = Regex::new(r"(?i)^(?:[-*]\s*)?(?:decision|truth)\s*:\s*(.+)$")
        .map_err(|err| TruthError::Invalid(err.to_string()))?;

    for task in load_tasks(backlog_dir) {
        let source_path = task
            .file_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| "(unknown task file)".to_string());
        for line in task.body.lines() {
            let trimmed = line.trim();
            let Some(caps) = decision_re.captures(trimmed) else {
                continue;
            };
            let Some(statement) = caps.get(1).map(|m| m.as_str().trim()) else {
                continue;
            };
            if statement.is_empty() {
                continue;
            }
            let fingerprint = legacy_fingerprint("task_note", &task.id, statement);
            candidates.push(TruthLegacyCandidate {
                source_type: "task_note".to_string(),
                source_id: task.id.clone(),
                source_path: source_path.clone(),
                statement: statement.to_string(),
                suggested_title: suggest_title(statement),
                fingerprint,
                context: TruthContext {
                    project_id: task.project.clone(),
                    epic_id: Some(task.id.clone()),
                    feature: Some(task.id.clone()),
                    session_id: None,
                    worktree_id: None,
                    worktree_path: None,
                },
            });
        }
    }

    match resolve_workmesh_home() {
        Ok(home) => match load_sessions_latest(&home) {
            Ok(sessions) => {
                let source_path = home
                    .join("sessions")
                    .join("events.jsonl")
                    .to_string_lossy()
                    .to_string();
                for session in sessions {
                    let decisions = session
                        .handoff
                        .as_ref()
                        .map(|handoff| handoff.decisions.clone())
                        .unwrap_or_default();
                    for decision in decisions {
                        let statement = decision.trim();
                        if statement.is_empty() {
                            continue;
                        }
                        let fingerprint =
                            legacy_fingerprint("session_handoff", &session.id, statement);
                        candidates.push(TruthLegacyCandidate {
                            source_type: "session_handoff".to_string(),
                            source_id: session.id.clone(),
                            source_path: source_path.clone(),
                            statement: statement.to_string(),
                            suggested_title: suggest_title(statement),
                            fingerprint,
                            context: TruthContext {
                                project_id: session.project_id.clone(),
                                epic_id: session.epic_id.clone(),
                                feature: session.epic_id.clone(),
                                session_id: Some(session.id.clone()),
                                worktree_id: session.worktree.as_ref().and_then(|w| w.id.clone()),
                                worktree_path: session
                                    .worktree
                                    .as_ref()
                                    .map(|w| w.path.clone()),
                            },
                        });
                    }
                }
            }
            Err(err) => warnings.push(format!("unable to scan global sessions: {}", err)),
        },
        Err(err) => warnings.push(format!("unable to resolve WORKMESH_HOME: {}", err)),
    }

    candidates.sort_by(|a, b| {
        a.source_type
            .cmp(&b.source_type)
            .then_with(|| a.source_id.cmp(&b.source_id))
            .then_with(|| a.statement.cmp(&b.statement))
    });

    Ok(TruthMigrationAudit {
        generated_at: now_rfc3339(),
        candidates,
        warnings,
    })
}

pub fn truth_migration_plan(
    backlog_dir: &Path,
    audit: &TruthMigrationAudit,
) -> Result<TruthMigrationPlan, TruthError> {
    let existing = project_from_events(backlog_dir)?;
    let migrated_fingerprints = existing
        .values()
        .flat_map(|record| record.tags.iter())
        .filter_map(|tag| tag.strip_prefix("legacy:"))
        .map(|value| value.to_string())
        .collect::<HashSet<_>>();

    let mut to_create = Vec::new();
    let mut skipped = Vec::new();

    for candidate in &audit.candidates {
        let reason;
        let action;
        if migrated_fingerprints.contains(&candidate.fingerprint) {
            action = "skip".to_string();
            reason = "already migrated".to_string();
            skipped.push(TruthMigrationPlanItem {
                candidate: candidate.clone(),
                action,
                reason,
            });
            continue;
        }

        action = "propose_truth".to_string();
        reason = "legacy decision candidate".to_string();
        to_create.push(TruthMigrationPlanItem {
            candidate: candidate.clone(),
            action,
            reason,
        });
    }

    Ok(TruthMigrationPlan {
        generated_at: now_rfc3339(),
        to_create,
        skipped,
        warnings: audit.warnings.clone(),
    })
}

pub fn apply_truth_migration(
    backlog_dir: &Path,
    plan: &TruthMigrationPlan,
    dry_run: bool,
) -> Result<TruthMigrationResult, TruthError> {
    if dry_run {
        return Ok(TruthMigrationResult {
            dry_run: true,
            created_ids: Vec::new(),
            skipped: plan
                .skipped
                .iter()
                .map(|item| format!("{}:{}", item.candidate.source_type, item.candidate.source_id))
                .collect(),
        });
    }

    let mut created_ids = Vec::new();
    let mut skipped = Vec::new();

    for item in &plan.to_create {
        let tags = vec![
            "migrated".to_string(),
            format!("legacy:{}", item.candidate.fingerprint),
            format!("source:{}", item.candidate.source_type),
        ];
        let record = propose_truth(
            backlog_dir,
            TruthProposeInput {
                id: None,
                title: item.candidate.suggested_title.clone(),
                statement: item.candidate.statement.clone(),
                rationale: Some(format!(
                    "Migrated from {}:{}",
                    item.candidate.source_type, item.candidate.source_id
                )),
                constraints: Vec::new(),
                tags,
                context: item.candidate.context.clone(),
                actor: Some("truth-migration".to_string()),
            },
        )?;
        created_ids.push(record.id);
    }

    for item in &plan.skipped {
        skipped.push(format!(
            "{}:{} ({})",
            item.candidate.source_type, item.candidate.source_id, item.reason
        ));
    }

    Ok(TruthMigrationResult {
        dry_run: false,
        created_ids,
        skipped,
    })
}

fn apply_transition(
    backlog_dir: &Path,
    input: TruthTransitionInput,
    accept: bool,
) -> Result<TruthRecord, TruthError> {
    ensure_truth_dirs(backlog_dir)?;
    let truth_id = normalized_non_empty("truth_id", &input.truth_id)?;

    let projected = project_from_events(backlog_dir)?;
    let existing = projected.get(&truth_id).ok_or_else(|| {
        TruthError::Invalid(format!("unknown truth id: {}", truth_id))
    })?;

    if existing.state != TruthState::Proposed {
        return Err(TruthError::Invalid(format!(
            "truth {} is {} and cannot transition",
            truth_id,
            existing.state.as_str()
        )));
    }

    let event = if accept {
        TruthEvent::Accepted {
            event_id: Ulid::new().to_string(),
            truth_id: truth_id.clone(),
            timestamp: now_rfc3339(),
            actor: normalize_optional(input.actor),
            note: normalize_optional(input.note),
        }
    } else {
        TruthEvent::Rejected {
            event_id: Ulid::new().to_string(),
            truth_id: truth_id.clone(),
            timestamp: now_rfc3339(),
            actor: normalize_optional(input.actor),
            reason: normalize_optional(input.note),
        }
    };

    append_truth_event(backlog_dir, &event)?;
    rebuild_truth_projection(backlog_dir)?;
    show_truth(backlog_dir, &truth_id)?.ok_or_else(|| {
        TruthError::Invalid(format!(
            "truth {} not found after transition write",
            truth_id
        ))
    })
}

fn append_truth_event(backlog_dir: &Path, event: &TruthEvent) -> Result<(), TruthError> {
    ensure_truth_dirs(backlog_dir)?;
    let path = truth_events_path(backlog_dir);
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    let line = serde_json::to_string(event)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

fn read_events_strict(backlog_dir: &Path) -> Result<Vec<TruthEvent>, TruthError> {
    let (events, malformed) = read_events_with_errors(backlog_dir)?;
    if malformed.is_empty() {
        Ok(events)
    } else {
        Err(TruthError::Invalid(format!(
            "truth events contain malformed lines: {}",
            malformed.join("; ")
        )))
    }
}

fn read_events_with_errors(
    backlog_dir: &Path,
) -> Result<(Vec<TruthEvent>, Vec<String>), TruthError> {
    let path = truth_events_path(backlog_dir);
    if !path.exists() {
        return Ok((Vec::new(), Vec::new()));
    }

    let file = fs::File::open(&path)?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    let mut malformed = Vec::new();

    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<TruthEvent>(trimmed) {
            Ok(event) => events.push(event),
            Err(err) => malformed.push(format!("line {}: {}", idx + 1, err)),
        }
    }

    Ok((events, malformed))
}

fn project_from_events(backlog_dir: &Path) -> Result<BTreeMap<String, TruthRecord>, TruthError> {
    let events = read_events_strict(backlog_dir)?;
    project_events(&events)
}

fn project_events(events: &[TruthEvent]) -> Result<BTreeMap<String, TruthRecord>, TruthError> {
    let mut projected = BTreeMap::<String, TruthRecord>::new();
    for event in events {
        apply_event(&mut projected, event).map_err(TruthError::Invalid)?;
    }
    Ok(projected)
}

fn apply_event(
    projected: &mut BTreeMap<String, TruthRecord>,
    event: &TruthEvent,
) -> Result<(), String> {
    match event {
        TruthEvent::Proposed {
            truth_id,
            timestamp,
            title,
            statement,
            rationale,
            constraints,
            tags,
            context,
            ..
        } => {
            let key = truth_id.trim().to_string();
            if key.is_empty() {
                return Err("proposed event has empty truth_id".to_string());
            }
            if projected.contains_key(&key) {
                return Err(format!("duplicate proposed event for {}", key));
            }
            let record = TruthRecord {
                id: key.clone(),
                created_at: timestamp.to_string(),
                updated_at: timestamp.to_string(),
                version: 1,
                state: TruthState::Proposed,
                title: title.trim().to_string(),
                statement: statement.trim().to_string(),
                rationale: normalize_optional(rationale.clone()),
                constraints: normalize_list(constraints.clone()),
                tags: normalize_list(tags.clone()),
                context: normalize_context(context.clone()),
                accepted_at: None,
                rejected_at: None,
                superseded_at: None,
                superseded_by: None,
                history: vec![history_entry(event)],
            };
            projected.insert(key, record);
        }
        TruthEvent::Accepted { truth_id, .. } => {
            let Some(record) = projected.get_mut(truth_id) else {
                return Err(format!("accepted event references unknown truth {}", truth_id));
            };
            if record.state != TruthState::Proposed {
                return Err(format!(
                    "truth {} is {} and cannot be accepted",
                    truth_id,
                    record.state.as_str()
                ));
            }
            record.state = TruthState::Accepted;
            record.updated_at = event.timestamp().to_string();
            record.accepted_at = Some(event.timestamp().to_string());
            record.version += 1;
            record.history.push(history_entry(event));
        }
        TruthEvent::Rejected { truth_id, .. } => {
            let Some(record) = projected.get_mut(truth_id) else {
                return Err(format!("rejected event references unknown truth {}", truth_id));
            };
            if record.state != TruthState::Proposed {
                return Err(format!(
                    "truth {} is {} and cannot be rejected",
                    truth_id,
                    record.state.as_str()
                ));
            }
            record.state = TruthState::Rejected;
            record.updated_at = event.timestamp().to_string();
            record.rejected_at = Some(event.timestamp().to_string());
            record.version += 1;
            record.history.push(history_entry(event));
        }
        TruthEvent::Superseded {
            truth_id,
            by_truth_id,
            ..
        } => {
            if truth_id.eq_ignore_ascii_case(by_truth_id) {
                return Err(format!(
                    "truth {} cannot supersede itself",
                    truth_id
                ));
            }
            let Some(replacement) = projected.get(by_truth_id) else {
                return Err(format!(
                    "superseded event references missing replacement truth {}",
                    by_truth_id
                ));
            };
            if replacement.state != TruthState::Accepted {
                return Err(format!(
                    "replacement truth {} is {} and cannot supersede",
                    by_truth_id,
                    replacement.state.as_str()
                ));
            }
            let Some(record) = projected.get_mut(truth_id) else {
                return Err(format!("superseded event references unknown truth {}", truth_id));
            };
            if record.state != TruthState::Accepted {
                return Err(format!(
                    "truth {} is {} and cannot be superseded",
                    truth_id,
                    record.state.as_str()
                ));
            }
            record.state = TruthState::Superseded;
            record.updated_at = event.timestamp().to_string();
            record.superseded_at = Some(event.timestamp().to_string());
            record.superseded_by = Some(by_truth_id.to_string());
            record.version += 1;
            record.history.push(history_entry(event));
        }
    }
    Ok(())
}

fn history_entry(event: &TruthEvent) -> TruthHistoryEntry {
    let (note, by_truth_id) = match event {
        TruthEvent::Accepted { note, .. } => (note.clone(), None),
        TruthEvent::Rejected { reason, .. } => (reason.clone(), None),
        TruthEvent::Superseded {
            reason,
            by_truth_id,
            ..
        } => (reason.clone(), Some(by_truth_id.clone())),
        TruthEvent::Proposed { .. } => (None, None),
    };

    TruthHistoryEntry {
        event_id: event.event_id().to_string(),
        timestamp: event.timestamp().to_string(),
        event_type: event.kind().to_string(),
        actor: event.actor(),
        note,
        by_truth_id,
    }
}

fn write_current_projection<'a, I>(backlog_dir: &Path, records: I) -> Result<(), TruthError>
where
    I: Iterator<Item = &'a TruthRecord>,
{
    ensure_truth_dirs(backlog_dir)?;

    let mut ordered = records.cloned().collect::<Vec<_>>();
    ordered.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.id.to_lowercase().cmp(&b.id.to_lowercase()))
    });

    let path = truth_current_path(backlog_dir);
    let tmp = path.with_extension("jsonl.tmp");
    let mut file = fs::File::create(&tmp)?;
    for record in ordered {
        let line = serde_json::to_string(&record)?;
        writeln!(file, "{}", line)?;
    }
    fs::rename(&tmp, &path)?;
    Ok(())
}

fn read_current_records_with_errors(
    backlog_dir: &Path,
    transition_errors: &mut Vec<String>,
) -> Result<Vec<TruthRecord>, TruthError> {
    let path = truth_current_path(backlog_dir);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(&path)?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<TruthRecord>(trimmed) {
            Ok(record) => records.push(record),
            Err(err) => transition_errors.push(format!(
                "current projection line {} malformed: {}",
                idx + 1,
                err
            )),
        }
    }
    Ok(records)
}

fn compare_projection(
    projected: &BTreeMap<String, TruthRecord>,
    current_records: &[TruthRecord],
) -> Vec<String> {
    let current = current_records
        .iter()
        .map(|record| (record.id.to_string(), record))
        .collect::<BTreeMap<_, _>>();

    let mut mismatches = Vec::new();

    for (id, record) in projected {
        match current.get(id) {
            None => mismatches.push(format!("missing_in_current:{}", id)),
            Some(existing) => {
                if record.state != existing.state
                    || record.version != existing.version
                    || record.updated_at != existing.updated_at
                    || record.superseded_by != existing.superseded_by
                {
                    mismatches.push(format!("mismatch:{}", id));
                }
            }
        }
    }

    for id in current.keys() {
        if !projected.contains_key(id) {
            mismatches.push(format!("extra_in_current:{}", id));
        }
    }

    mismatches
}

fn matches_query(record: &TruthRecord, query: &TruthQuery) -> bool {
    if !query.states.is_empty() && !query.states.iter().any(|state| *state == record.state) {
        return false;
    }

    if !matches_opt(&record.context.project_id, query.project_id.as_deref()) {
        return false;
    }
    if !matches_opt(&record.context.epic_id, query.epic_id.as_deref()) {
        return false;
    }
    if !matches_opt(&record.context.feature, query.feature.as_deref()) {
        return false;
    }
    if !matches_opt(&record.context.session_id, query.session_id.as_deref()) {
        return false;
    }
    if !matches_opt(&record.context.worktree_id, query.worktree_id.as_deref()) {
        return false;
    }
    if !matches_opt(&record.context.worktree_path, query.worktree_path.as_deref()) {
        return false;
    }

    if !query.tags.is_empty() {
        let have = record
            .tags
            .iter()
            .map(|tag| tag.to_lowercase())
            .collect::<HashSet<_>>();
        for tag in &query.tags {
            if !have.contains(&tag.to_lowercase()) {
                return false;
            }
        }
    }

    true
}

fn legacy_fingerprint(source_type: &str, source_id: &str, text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source_type.as_bytes());
    hasher.update(b"|");
    hasher.update(source_id.as_bytes());
    hasher.update(b"|");
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn suggest_title(statement: &str) -> String {
    let normalized = statement.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return "Migrated decision".to_string();
    }
    if normalized.len() > 72 {
        format!("{}...", &normalized[..69])
    } else {
        normalized
    }
}

fn now_rfc3339() -> String {
    Local::now().to_rfc3339()
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
}

fn normalize_list(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for value in values {
        let trimmed = value.trim();
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

fn normalize_context(mut context: TruthContext) -> TruthContext {
    context.project_id = normalize_optional(context.project_id);
    context.epic_id = normalize_optional(context.epic_id);
    context.feature = normalize_optional(context.feature);
    context.session_id = normalize_optional(context.session_id);
    context.worktree_id = normalize_optional(context.worktree_id);
    context.worktree_path = normalize_optional(context.worktree_path);
    context
}

fn normalized_non_empty(field: &str, value: &str) -> Result<String, TruthError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(TruthError::Invalid(format!("{} is required", field)));
    }
    Ok(trimmed.to_string())
}

fn matches_opt(have: &Option<String>, want: Option<&str>) -> bool {
    match want {
        None => true,
        Some(expected) => {
            let expected = expected.trim();
            if expected.is_empty() {
                return true;
            }
            have.as_deref()
                .map(|value| value.eq_ignore_ascii_case(expected))
                .unwrap_or(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_seed_task(backlog_dir: &Path) {
        let tasks_dir = backlog_dir.join("tasks");
        fs::create_dir_all(&tasks_dir).expect("tasks dir");
        fs::write(
            tasks_dir.join("task-main-001 - seed.md"),
            "---\nid: task-main-001\ntitle: Seed\nstatus: In Progress\npriority: P1\nphase: Phase1\ndependencies: []\nlabels: []\nassignee: []\n---\n\n## Notes\n- Decision: use event sourcing\n",
        )
        .expect("seed task");
    }

    #[test]
    fn propose_accept_and_query_by_context() {
        let temp = TempDir::new().expect("tempdir");
        let backlog = temp.path();

        let proposed = propose_truth(
            backlog,
            TruthProposeInput {
                id: Some("truth-001".to_string()),
                title: "Use append-only truth events".to_string(),
                statement: "Truth records must be append-only.".to_string(),
                rationale: Some("Keeps history immutable".to_string()),
                constraints: vec!["No in-place edits".to_string()],
                tags: vec!["architecture".to_string()],
                context: TruthContext {
                    project_id: Some("workmesh".to_string()),
                    epic_id: Some("task-main-001".to_string()),
                    feature: Some("truth-ledger".to_string()),
                    session_id: Some("01KTESTSESSION".to_string()),
                    worktree_id: Some("01KWORKTREE".to_string()),
                    worktree_path: Some("/tmp/worktree-a".to_string()),
                },
                actor: Some("test".to_string()),
            },
        )
        .expect("propose");

        assert_eq!(proposed.state, TruthState::Proposed);

        let accepted = accept_truth(
            backlog,
            TruthTransitionInput {
                truth_id: "truth-001".to_string(),
                note: Some("approved".to_string()),
                actor: Some("reviewer".to_string()),
            },
        )
        .expect("accept");

        assert_eq!(accepted.state, TruthState::Accepted);

        let listed = list_truths(
            backlog,
            &TruthQuery {
                states: vec![TruthState::Accepted],
                project_id: Some("workmesh".to_string()),
                epic_id: Some("task-main-001".to_string()),
                feature: Some("truth-ledger".to_string()),
                session_id: None,
                worktree_id: None,
                worktree_path: None,
                tags: vec!["architecture".to_string()],
                limit: None,
            },
        )
        .expect("list");

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "truth-001");
        assert_eq!(listed[0].state, TruthState::Accepted);
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let temp = TempDir::new().expect("tempdir");
        let backlog = temp.path();

        let _ = propose_truth(
            backlog,
            TruthProposeInput {
                id: Some("truth-001".to_string()),
                title: "One".to_string(),
                statement: "Statement".to_string(),
                rationale: None,
                constraints: Vec::new(),
                tags: Vec::new(),
                context: TruthContext::default(),
                actor: None,
            },
        )
        .expect("propose");

        let _ = reject_truth(
            backlog,
            TruthTransitionInput {
                truth_id: "truth-001".to_string(),
                note: Some("not now".to_string()),
                actor: None,
            },
        )
        .expect("reject");

        let err = accept_truth(
            backlog,
            TruthTransitionInput {
                truth_id: "truth-001".to_string(),
                note: None,
                actor: None,
            },
        )
        .expect_err("invalid");

        assert!(err.to_string().contains("cannot transition"));
    }

    #[test]
    fn supersede_requires_accepted_replacement() {
        let temp = TempDir::new().expect("tempdir");
        let backlog = temp.path();

        let _ = propose_truth(
            backlog,
            TruthProposeInput {
                id: Some("truth-old".to_string()),
                title: "Old".to_string(),
                statement: "Old statement".to_string(),
                rationale: None,
                constraints: Vec::new(),
                tags: Vec::new(),
                context: TruthContext::default(),
                actor: None,
            },
        )
        .expect("propose old");

        let _ = accept_truth(
            backlog,
            TruthTransitionInput {
                truth_id: "truth-old".to_string(),
                note: None,
                actor: None,
            },
        )
        .expect("accept old");

        let _ = propose_truth(
            backlog,
            TruthProposeInput {
                id: Some("truth-new".to_string()),
                title: "New".to_string(),
                statement: "New statement".to_string(),
                rationale: None,
                constraints: Vec::new(),
                tags: Vec::new(),
                context: TruthContext::default(),
                actor: None,
            },
        )
        .expect("propose new");

        let err = supersede_truth(
            backlog,
            TruthSupersedeInput {
                truth_id: "truth-old".to_string(),
                by_truth_id: "truth-new".to_string(),
                reason: Some("replacement still proposed".to_string()),
                actor: None,
            },
        )
        .expect_err("must fail");
        assert!(err.to_string().contains("must be accepted"));

        let _ = accept_truth(
            backlog,
            TruthTransitionInput {
                truth_id: "truth-new".to_string(),
                note: None,
                actor: None,
            },
        )
        .expect("accept new");

        let superseded = supersede_truth(
            backlog,
            TruthSupersedeInput {
                truth_id: "truth-old".to_string(),
                by_truth_id: "truth-new".to_string(),
                reason: Some("adopt replacement".to_string()),
                actor: Some("architect".to_string()),
            },
        )
        .expect("supersede");

        assert_eq!(superseded.state, TruthState::Superseded);
        assert_eq!(superseded.superseded_by.as_deref(), Some("truth-new"));
    }

    #[test]
    fn validate_reports_projection_mismatch() {
        let temp = TempDir::new().expect("tempdir");
        let backlog = temp.path();

        let _ = propose_truth(
            backlog,
            TruthProposeInput {
                id: Some("truth-001".to_string()),
                title: "A".to_string(),
                statement: "B".to_string(),
                rationale: None,
                constraints: Vec::new(),
                tags: Vec::new(),
                context: TruthContext::default(),
                actor: None,
            },
        )
        .expect("propose");

        let current_path = truth_current_path(backlog);
        fs::write(
            &current_path,
            r#"{"id":"truth-001","created_at":"x","updated_at":"x","version":99,"state":"accepted","title":"A","statement":"B","constraints":[],"tags":[],"context":{},"history":[]}"#,
        )
        .expect("tamper");

        let report = validate_truth_store(backlog).expect("validate");
        assert!(!report.ok);
        assert!(
            report
                .projection_mismatches
                .iter()
                .any(|line| line.contains("mismatch:truth-001"))
        );
    }

    #[test]
    fn migration_audit_plan_and_apply_work() {
        let temp = TempDir::new().expect("tempdir");
        let backlog = temp.path();
        write_seed_task(backlog);

        let audit = truth_migration_audit(backlog).expect("audit");
        assert!(!audit.candidates.is_empty());

        let plan = truth_migration_plan(backlog, &audit).expect("plan");
        assert!(!plan.to_create.is_empty());

        let dry_run = apply_truth_migration(backlog, &plan, true).expect("dry-run");
        assert!(dry_run.created_ids.is_empty());

        let applied = apply_truth_migration(backlog, &plan, false).expect("apply");
        assert!(!applied.created_ids.is_empty());

        let listed = list_truths(
            backlog,
            &TruthQuery {
                states: vec![TruthState::Proposed],
                ..TruthQuery::default()
            },
        )
        .expect("list");
        assert!(!listed.is_empty());
    }
}
