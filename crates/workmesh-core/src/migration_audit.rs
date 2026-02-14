use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::backlog::{resolve_backlog, BacklogError, BacklogLayout, BacklogResolution};
use crate::config::{load_config, update_do_not_migrate, ConfigError};
use crate::context::{context_from_legacy_focus, context_path, load_context, save_context};
use crate::focus::{focus_path, load_focus};
use crate::global_sessions::{
    append_session_saved, load_sessions_latest, rebuild_sessions_index, resolve_workmesh_home,
    set_current_session, AgentSession, HandoffSummary,
};
use crate::migration::{migrate_backlog, MigrationError};
use crate::truth::{apply_truth_migration, truth_migration_audit, truth_migration_plan};

#[derive(Debug, Error)]
pub enum MigrationAuditError {
    #[error("Backlog resolution failed: {0}")]
    Backlog(#[from] BacklogError),
    #[error("Migration failed: {0}")]
    Migration(#[from] MigrationError),
    #[error("Context conversion failed: {0}")]
    Context(#[from] anyhow::Error),
    #[error("Config update failed: {0}")]
    Config(#[from] ConfigError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationActionKind {
    LayoutBacklogToWorkmesh,
    FocusToContext,
    TruthBackfill,
    SessionHandoffEnrichment,
    ConfigCleanup,
}

impl MigrationActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LayoutBacklogToWorkmesh => "layout_backlog_to_workmesh",
            Self::FocusToContext => "focus_to_context",
            Self::TruthBackfill => "truth_backfill",
            Self::SessionHandoffEnrichment => "session_handoff_enrichment",
            Self::ConfigCleanup => "config_cleanup",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "layout_backlog_to_workmesh" => Some(Self::LayoutBacklogToWorkmesh),
            "focus_to_context" => Some(Self::FocusToContext),
            "truth_backfill" => Some(Self::TruthBackfill),
            "session_handoff_enrichment" => Some(Self::SessionHandoffEnrichment),
            "config_cleanup" => Some(Self::ConfigCleanup),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationFinding {
    pub id: String,
    pub title: String,
    pub severity: String,
    pub details: serde_json::Value,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationAuditReport {
    pub repo_root: String,
    pub backlog_dir: String,
    pub layout: String,
    pub findings: Vec<MigrationFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlanStep {
    pub action: String,
    pub required: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub repo_root: String,
    pub steps: Vec<MigrationPlanStep>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MigrationPlanOptions {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl Default for MigrationPlanOptions {
    fn default() -> Self {
        Self {
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MigrationApplyOptions {
    pub dry_run: bool,
    pub backup: bool,
}

impl Default for MigrationApplyOptions {
    fn default() -> Self {
        Self {
            dry_run: true,
            backup: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationApplyResult {
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
    pub warnings: Vec<String>,
    pub backups: Vec<String>,
}

pub fn layout_name(layout: BacklogLayout) -> &'static str {
    match layout {
        BacklogLayout::Workmesh => "workmesh",
        BacklogLayout::HiddenWorkmesh => ".workmesh",
        BacklogLayout::Backlog => "backlog",
        BacklogLayout::Project => "project",
        BacklogLayout::RootTasks => "root/tasks",
        BacklogLayout::TasksDir => "tasks-dir",
        BacklogLayout::Custom => "custom",
    }
}

pub fn audit_deprecations(root: &Path) -> Result<MigrationAuditReport, MigrationAuditError> {
    let resolution = resolve_backlog(root)?;
    let mut findings = Vec::new();

    if resolution.layout.is_legacy() {
        findings.push(MigrationFinding {
            id: "legacy_layout".to_string(),
            title: "Legacy backlog layout detected".to_string(),
            severity: "required".to_string(),
            details: serde_json::json!({
                "layout": layout_name(resolution.layout),
                "from": resolution.backlog_dir,
                "target": resolution.repo_root.join("workmesh"),
            }),
            suggested_action: Some(MigrationActionKind::LayoutBacklogToWorkmesh.as_str().into()),
        });
    }

    let legacy_focus_path = focus_path(&resolution.backlog_dir);
    let has_legacy_focus = legacy_focus_path.exists();
    let has_context = context_path(&resolution.backlog_dir).exists();
    if has_legacy_focus {
        findings.push(MigrationFinding {
            id: "legacy_focus".to_string(),
            title: "Deprecated focus.json detected".to_string(),
            severity: "required".to_string(),
            details: serde_json::json!({
                "path": legacy_focus_path,
                "replacement": context_path(&resolution.backlog_dir),
            }),
            suggested_action: Some(MigrationActionKind::FocusToContext.as_str().into()),
        });
    } else if !has_context {
        findings.push(MigrationFinding {
            id: "missing_context".to_string(),
            title: "No context.json found".to_string(),
            severity: "recommended".to_string(),
            details: serde_json::json!({
                "path": context_path(&resolution.backlog_dir),
            }),
            suggested_action: Some(MigrationActionKind::FocusToContext.as_str().into()),
        });
    }

    if let Some(config) = load_config(&resolution.repo_root) {
        if config.do_not_migrate.unwrap_or(false) {
            findings.push(MigrationFinding {
                id: "deprecated_config_do_not_migrate".to_string(),
                title: "Deprecated do_not_migrate=true config found".to_string(),
                severity: "recommended".to_string(),
                details: serde_json::json!({
                    "config_root": resolution.repo_root,
                }),
                suggested_action: Some(MigrationActionKind::ConfigCleanup.as_str().into()),
            });
        }
    }

    if let Ok(truth_audit) = truth_migration_audit(&resolution.backlog_dir) {
        if !truth_audit.candidates.is_empty() {
            findings.push(MigrationFinding {
                id: "legacy_truth_candidates".to_string(),
                title: "Legacy decision notes found for Truth Ledger backfill".to_string(),
                severity: "recommended".to_string(),
                details: serde_json::json!({
                    "candidate_count": truth_audit.candidates.len(),
                }),
                suggested_action: Some(MigrationActionKind::TruthBackfill.as_str().into()),
            });
        }
    }

    if let Ok(home) = resolve_workmesh_home() {
        if let Ok(sessions) = load_sessions_latest(&home) {
            let missing = sessions.iter().filter(|s| s.handoff.is_none()).count();
            if missing > 0 {
                findings.push(MigrationFinding {
                    id: "legacy_sessions_missing_handoff".to_string(),
                    title: "Global sessions missing structured handoff".to_string(),
                    severity: "recommended".to_string(),
                    details: serde_json::json!({
                        "home": home,
                        "missing_count": missing,
                    }),
                    suggested_action: Some(
                        MigrationActionKind::SessionHandoffEnrichment
                            .as_str()
                            .into(),
                    ),
                });
            }
        }
    }

    Ok(MigrationAuditReport {
        repo_root: resolution.repo_root.to_string_lossy().to_string(),
        backlog_dir: resolution.backlog_dir.to_string_lossy().to_string(),
        layout: layout_name(resolution.layout).to_string(),
        findings,
    })
}

pub fn plan_migrations(
    report: &MigrationAuditReport,
    opts: &MigrationPlanOptions,
) -> MigrationPlan {
    let include: HashSet<MigrationActionKind> = opts
        .include
        .iter()
        .filter_map(|v| MigrationActionKind::from_str(v))
        .collect();
    let exclude: HashSet<MigrationActionKind> = opts
        .exclude
        .iter()
        .filter_map(|v| MigrationActionKind::from_str(v))
        .collect();

    let mut wants = Vec::<MigrationActionKind>::new();
    for finding in &report.findings {
        if let Some(action) = finding
            .suggested_action
            .as_deref()
            .and_then(MigrationActionKind::from_str)
        {
            if !wants.contains(&action) {
                wants.push(action);
            }
        }
    }

    let order = [
        MigrationActionKind::LayoutBacklogToWorkmesh,
        MigrationActionKind::FocusToContext,
        MigrationActionKind::TruthBackfill,
        MigrationActionKind::SessionHandoffEnrichment,
        MigrationActionKind::ConfigCleanup,
    ];

    let mut steps = Vec::new();
    let mut warnings = Vec::new();
    for action in order {
        if !wants.contains(&action) {
            continue;
        }
        if !include.is_empty() && !include.contains(&action) {
            continue;
        }
        if exclude.contains(&action) {
            warnings.push(format!("excluded action {}", action.as_str()));
            continue;
        }
        let required = matches!(
            action,
            MigrationActionKind::LayoutBacklogToWorkmesh | MigrationActionKind::FocusToContext
        );
        steps.push(MigrationPlanStep {
            action: action.as_str().to_string(),
            required,
            reason: reason_for_action(action).to_string(),
        });
    }

    MigrationPlan {
        repo_root: report.repo_root.clone(),
        steps,
        warnings,
    }
}

fn reason_for_action(action: MigrationActionKind) -> &'static str {
    match action {
        MigrationActionKind::LayoutBacklogToWorkmesh => "normalize legacy backlog layout",
        MigrationActionKind::FocusToContext => {
            "replace deprecated focus orchestration with context.json"
        }
        MigrationActionKind::TruthBackfill => {
            "backfill legacy decision notes into structured truth records"
        }
        MigrationActionKind::SessionHandoffEnrichment => {
            "enrich global sessions with structured handoff fields"
        }
        MigrationActionKind::ConfigCleanup => "remove deprecated migration suppression flag",
    }
}

pub fn apply_migration_plan(
    root: &Path,
    plan: &MigrationPlan,
    opts: &MigrationApplyOptions,
) -> Result<MigrationApplyResult, MigrationAuditError> {
    let mut result = MigrationApplyResult {
        applied: Vec::new(),
        skipped: Vec::new(),
        warnings: plan.warnings.clone(),
        backups: Vec::new(),
    };

    for step in &plan.steps {
        let Some(kind) = MigrationActionKind::from_str(&step.action) else {
            result
                .warnings
                .push(format!("unknown action {}", step.action));
            result.skipped.push(step.action.clone());
            continue;
        };
        match kind {
            MigrationActionKind::LayoutBacklogToWorkmesh => {
                if opts.dry_run {
                    result.applied.push(format!("{} (dry-run)", kind.as_str()));
                } else {
                    let resolution = resolve_backlog(root)?;
                    if resolution.layout.is_legacy() {
                        let _ = migrate_backlog(&resolution, "workmesh")?;
                    }
                    result.applied.push(kind.as_str().to_string());
                }
            }
            MigrationActionKind::FocusToContext => {
                if opts.dry_run {
                    result.applied.push(format!("{} (dry-run)", kind.as_str()));
                } else {
                    let res = resolve_backlog(root)?;
                    apply_focus_to_context(&res, opts, &mut result)?;
                    result.applied.push(kind.as_str().to_string());
                }
            }
            MigrationActionKind::SessionHandoffEnrichment => {
                if opts.dry_run {
                    result.applied.push(format!("{} (dry-run)", kind.as_str()));
                } else {
                    enrich_sessions_handoff(&mut result)?;
                    result.applied.push(kind.as_str().to_string());
                }
            }
            MigrationActionKind::TruthBackfill => {
                if opts.dry_run {
                    result.applied.push(format!("{} (dry-run)", kind.as_str()));
                } else {
                    let res = resolve_backlog(root)?;
                    let audit =
                        truth_migration_audit(&res.backlog_dir).map_err(std::io::Error::other)?;
                    let plan = truth_migration_plan(&res.backlog_dir, &audit)
                        .map_err(std::io::Error::other)?;
                    let migration = apply_truth_migration(&res.backlog_dir, &plan, false)
                        .map_err(std::io::Error::other)?;
                    if migration.created_ids.is_empty() {
                        result
                            .warnings
                            .push("truth_backfill: no legacy candidates to migrate".to_string());
                    }
                    result.applied.push(kind.as_str().to_string());
                }
            }
            MigrationActionKind::ConfigCleanup => {
                if opts.dry_run {
                    result.applied.push(format!("{} (dry-run)", kind.as_str()));
                } else {
                    let res = resolve_backlog(root)?;
                    if load_config(&res.repo_root)
                        .and_then(|c| c.do_not_migrate)
                        .unwrap_or(false)
                    {
                        let _ = update_do_not_migrate(&res.repo_root, false)?;
                        result.applied.push(kind.as_str().to_string());
                    } else {
                        result.skipped.push(kind.as_str().to_string());
                    }
                }
            }
        }
    }
    Ok(result)
}

fn apply_focus_to_context(
    resolution: &BacklogResolution,
    opts: &MigrationApplyOptions,
    result: &mut MigrationApplyResult,
) -> Result<(), MigrationAuditError> {
    let focus_file = focus_path(&resolution.backlog_dir);
    let has_focus = focus_file.exists();
    let existing_context = load_context(&resolution.backlog_dir)?;
    if existing_context.is_some() && !has_focus {
        result
            .skipped
            .push(MigrationActionKind::FocusToContext.as_str().into());
        return Ok(());
    }

    if has_focus && opts.backup {
        let backup_dir = resolution
            .backlog_dir
            .join("migrations")
            .join(now_compact_timestamp());
        fs::create_dir_all(&backup_dir)?;
        let backup_path = backup_dir.join("focus.json.bak");
        fs::copy(&focus_file, &backup_path)?;
        result
            .backups
            .push(backup_path.to_string_lossy().to_string());
    }

    let context = if has_focus {
        let legacy = load_focus(&resolution.backlog_dir)?;
        if let Some(state) = legacy {
            context_from_legacy_focus(
                state.project_id,
                state.epic_id,
                state.objective,
                state.working_set,
            )
        } else {
            context_from_legacy_focus(None, None, None, Vec::new())
        }
    } else {
        context_from_legacy_focus(None, None, None, Vec::new())
    };
    let _ = save_context(&resolution.backlog_dir, context)?;
    if has_focus {
        fs::remove_file(&focus_file)?;
    }
    Ok(())
}

fn enrich_sessions_handoff(result: &mut MigrationApplyResult) -> Result<(), MigrationAuditError> {
    let home = resolve_workmesh_home().map_err(std::io::Error::other)?;
    let sessions = load_sessions_latest(&home).map_err(std::io::Error::other)?;
    let mut changed = 0usize;
    for session in sessions {
        if session.handoff.is_some() {
            continue;
        }
        let mut updated: AgentSession = session.clone();
        updated.handoff = Some(HandoffSummary::default());
        updated.updated_at = crate::global_sessions::now_rfc3339();
        append_session_saved(&home, updated)?;
        changed += 1;
    }
    if changed > 0 {
        let _ = rebuild_sessions_index(&home).map_err(std::io::Error::other)?;
        if let Some(current_id) = crate::global_sessions::read_current_session_id(&home) {
            let _ = set_current_session(&home, &current_id).map_err(std::io::Error::other)?;
        }
    } else {
        result
            .warnings
            .push("session_handoff_enrichment: nothing to enrich".to_string());
    }
    Ok(())
}

fn now_compact_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d%H%M%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_task(tasks_dir: &Path) {
        let _ = fs::create_dir_all(tasks_dir);
        let _ = fs::write(
            tasks_dir.join("task-001 - seed.md"),
            "---\nid: task-001\ntitle: Seed\nstatus: To Do\npriority: P2\nphase: Phase1\n---\n",
        );
    }

    #[test]
    fn audit_detects_legacy_layout_and_focus() {
        let temp = TempDir::new().expect("tempdir");
        let backlog = temp.path().join("backlog").join("tasks");
        write_task(&backlog);
        let _ = fs::write(
            temp.path().join("backlog").join("focus.json"),
            r#"{"project_id":"demo","epic_id":"task-001","objective":"Ship","working_set":["task-001"]}"#,
        );

        let report = audit_deprecations(temp.path()).expect("audit");
        let ids: Vec<String> = report.findings.into_iter().map(|f| f.id).collect();
        assert!(ids.contains(&"legacy_layout".to_string()));
        assert!(ids.contains(&"legacy_focus".to_string()));
    }

    #[test]
    fn apply_focus_to_context_removes_focus_and_creates_context() {
        let temp = TempDir::new().expect("tempdir");
        let tasks = temp.path().join("workmesh").join("tasks");
        write_task(&tasks);
        let backlog = temp.path().join("workmesh");
        let _ = fs::write(
            backlog.join("focus.json"),
            r#"{"project_id":"demo","epic_id":"task-001","objective":"Ship","working_set":["task-001"]}"#,
        );
        let report = audit_deprecations(temp.path()).expect("audit");
        let plan = plan_migrations(&report, &MigrationPlanOptions::default());
        let result = apply_migration_plan(
            temp.path(),
            &plan,
            &MigrationApplyOptions {
                dry_run: false,
                backup: true,
            },
        )
        .expect("apply");
        assert!(result
            .applied
            .iter()
            .any(|step| step.contains("focus_to_context")));
        assert!(backlog.join("context.json").is_file());
        assert!(!backlog.join("focus.json").exists());
    }
}
