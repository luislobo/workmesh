use std::collections::HashSet;
use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{Duration, Local, NaiveDate};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};

mod version;

use workmesh_core::archive::{archive_tasks, ArchiveOptions};
use workmesh_core::audit::{append_audit_event, AuditEvent};
use workmesh_core::backlog::{locate_backlog_dir, resolve_backlog, BacklogResolution};
use workmesh_core::config::update_do_not_migrate;
use workmesh_core::context::{
    clear_context, context_path, extract_task_id_from_branch, infer_project_id, load_context,
    save_context, ContextScope, ContextScopeMode, ContextState,
};
use workmesh_core::doctor::doctor_report;
use workmesh_core::fix::{backfill_missing_uids, fix_dependencies, FixerKind};
use workmesh_core::focus::load_focus;
use workmesh_core::gantt::{
    plantuml_gantt, render_plantuml_svg, write_text_file, PlantumlRenderError,
};
use workmesh_core::global_sessions::{
    append_session_saved, load_sessions_latest_fast, new_session_id, now_rfc3339,
    read_current_session_id, rebuild_sessions_index, refresh_sessions_index, resolve_workmesh_home,
    set_current_session, verify_sessions_index, AgentSession, CheckpointRef, GitSnapshot,
};
use workmesh_core::id_fix::{fix_duplicate_task_ids, FixIdsOptions};
use workmesh_core::index::{rebuild_index, refresh_index, verify_index};
use workmesh_core::initiative::{
    best_effort_git_branch as core_git_branch, ensure_branch_initiative, next_namespaced_task_id,
};
use workmesh_core::migration::{migrate_backlog, MigrationError};
use workmesh_core::migration_audit::{
    apply_migration_plan, audit_deprecations, plan_migrations, MigrationApplyOptions,
    MigrationPlanOptions,
};
use workmesh_core::project::{ensure_project_docs, repo_root_from_backlog};
use workmesh_core::quickstart::quickstart;
use workmesh_core::rekey::{
    parse_rekey_request, rekey_apply, render_rekey_prompt, RekeyApplyOptions, RekeyPromptOptions,
};
use workmesh_core::session::{
    append_session_journal, diff_since_checkpoint, render_diff, render_resume, resolve_project_id,
    resume_summary, task_summary, write_checkpoint, write_working_set, CheckpointOptions,
};
use workmesh_core::skills::{
    detect_user_agents, embedded_skill_ids, install_embedded_skill_global_auto_report,
    install_embedded_skill_report, load_skill_content, uninstall_embedded_skill_global_auto_report,
    uninstall_embedded_skill_report, SkillAgent, SkillInstallReport, SkillScope,
    SkillUninstallReport,
};
use workmesh_core::task::{load_tasks, load_tasks_with_archive, Lease, Task};
use workmesh_core::task_ops::{
    append_note, create_task_file, ensure_can_mark_done, filter_tasks, graph_export,
    is_lease_active, now_timestamp, ready_tasks, recommend_next_tasks_with_context,
    render_task_line, replace_section, set_list_field, sort_tasks, status_counts,
    task_to_json_value, tasks_to_json, tasks_to_jsonl, timestamp_plus_minutes, update_body,
    update_lease_fields, update_task_field, update_task_field_or_section, validate_tasks,
    FieldValue,
};
use workmesh_core::views::{
    blockers_report_with_context, board_lanes, scope_ids_from_context, BoardBy,
};

#[derive(Parser)]
#[command(name = "workmesh", version = version::FULL, about = "WorkMesh CLI (WIP)")]
struct Cli {
    /// Path to repo root or backlog directory
    #[arg(long, required = true)]
    root: PathBuf,
    /// Automatically write a checkpoint after mutating commands
    #[arg(long, action = ArgAction::SetTrue, global = true)]
    auto_checkpoint: bool,
    /// Automatically update the global agent session (requires an active session pointer)
    #[arg(long, action = ArgAction::SetTrue, global = true)]
    auto_session_save: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Diagnostics for repo layout, focus, index, and skill installation
    Doctor {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Install bundled WorkMesh skill packs (convenience command)
    Install {
        /// Install skill packs into agent skill directories
        #[arg(long, action = ArgAction::SetTrue)]
        skills: bool,
        /// Skill profile to install
        #[arg(long, value_enum, default_value_t = SkillProfileArg::All)]
        profile: SkillProfileArg,
        /// Install scope: project (default) or user
        #[arg(long, value_enum, default_value_t = SkillScopeArg::Project)]
        scope: SkillScopeArg,
        /// Which agent(s) to install for
        #[arg(long, value_enum, default_value_t = SkillAgentArg::All)]
        agent: SkillAgentArg,
        /// Overwrite existing SKILL.md files
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Uninstall bundled WorkMesh skill packs (convenience command)
    Uninstall {
        /// Uninstall skill packs from agent skill directories
        #[arg(long, action = ArgAction::SetTrue)]
        skills: bool,
        /// Skill profile to uninstall
        #[arg(long, value_enum, default_value_t = SkillProfileArg::All)]
        profile: SkillProfileArg,
        /// Uninstall scope: project (default) or user
        #[arg(long, value_enum, default_value_t = SkillScopeArg::Project)]
        scope: SkillScopeArg,
        /// Which agent(s) to uninstall for
        #[arg(long, value_enum, default_value_t = SkillAgentArg::All)]
        agent: SkillAgentArg,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Show a simple board view (swimlanes) grouped by status/phase/priority
    Board {
        /// Include archived tasks under `workmesh/archive/` (recursively)
        #[arg(long, action = ArgAction::SetTrue)]
        all: bool,
        /// Group lanes by this field
        #[arg(long, value_enum, default_value_t = BoardByArg::Status)]
        by: BoardByArg,
        /// Scope to the current focus (epic subtree or working set)
        #[arg(long, action = ArgAction::SetTrue)]
        focus: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Show blocked work and the top blockers (scoped to focus epic by default)
    Blockers {
        /// Include archived tasks under `workmesh/archive/` (recursively)
        #[arg(long, action = ArgAction::SetTrue)]
        all: bool,
        /// Override focus epic id for scoping
        #[arg(long)]
        epic_id: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// List tasks
    List {
        /// Include archived tasks under `workmesh/archive/` (recursively)
        #[arg(long, action = ArgAction::SetTrue)]
        all: bool,
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
    /// Fix duplicate task ids (after merges). By default this is a dry-run; pass --apply to write changes.
    FixIds {
        /// Apply changes (otherwise dry-run)
        #[arg(long, action = ArgAction::SetTrue)]
        apply: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Run fixers to detect/repair common task data issues
    Fix {
        #[command(subcommand)]
        command: FixCommand,
    },
    /// Generate an agent prompt to propose a task-id rekey mapping (and reference rewrites).
    RekeyPrompt {
        /// Include archived tasks under `workmesh/archive/` (recursively)
        #[arg(long, action = ArgAction::SetTrue)]
        all: bool,
        /// Include task bodies in the prompt data (can be large)
        #[arg(long, action = ArgAction::SetTrue)]
        include_body: bool,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Apply a task-id rekey mapping and rewrite structured references (dependencies + relationships).
    RekeyApply {
        /// Path to mapping JSON (if omitted, reads stdin)
        #[arg(long)]
        mapping: Option<PathBuf>,
        /// Apply changes (otherwise dry-run)
        #[arg(long, action = ArgAction::SetTrue)]
        apply: bool,
        /// Include archived tasks under `workmesh/archive/` (recursively)
        #[arg(long, action = ArgAction::SetTrue)]
        all: bool,
        /// Strict mode: only rewrites structured fields (id + dependencies + relationships), no task body edits.
        #[arg(long, action = ArgAction::SetTrue)]
        strict: bool,
        /// Non-strict mode (default): also rewrites free-text mentions of task IDs in task bodies.
        #[arg(long, action = ArgAction::SetTrue)]
        non_strict: bool,
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
    /// Global agent sessions (cross-repo continuity)
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    /// Repo-local context (project/objective/scope)
    Context {
        #[command(subcommand)]
        command: ContextCommand,
    },
    /// Deprecated alias for `context`
    Focus {
        #[command(subcommand)]
        command: ContextCommand,
    },
    /// Manage agent skills (show/install/uninstall)
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
    },
    /// Claim a task (lease)
    Claim {
        task_id: String,
        owner: String,
        #[arg(long)]
        minutes: Option<i64>,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
    },
    /// Release a task lease
    Release {
        task_id: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
    },
    /// Add label to task
    LabelAdd {
        task_id: String,
        label: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
    },
    /// Remove label from task
    LabelRemove {
        task_id: String,
        label: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
    },
    /// Add dependency to task
    DepAdd {
        task_id: String,
        dependency: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
    },
    /// Remove dependency from task
    DepRemove {
        task_id: String,
        dependency: String,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
    },
    /// Append a note to a task
    Note {
        task_id: String,
        note: String,
        #[arg(long, value_enum, default_value_t = NoteSection::Notes)]
        section: NoteSection,
        #[arg(long, action = ArgAction::SetTrue)]
        touch: bool,
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
    /// Migrate legacy/deprecated structures (audit -> plan -> apply)
    Migrate {
        #[command(subcommand)]
        command: Option<MigrateCommand>,
        /// Legacy migration target (compat mode when no subcommand is provided)
        #[arg(long)]
        to: Option<String>,
        /// Legacy non-interactive confirm (compat mode when no subcommand is provided)
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
enum SkillCommand {
    /// Show a skill's SKILL.md content (reads repo skill dirs, falls back to embedded default)
    Show {
        /// Skill name (defaults to workmesh)
        #[arg(long)]
        name: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Install the embedded WorkMesh skill into agent skill directories
    Install {
        /// Skill name (defaults to workmesh)
        #[arg(long)]
        name: Option<String>,
        /// Install to user-level (~/.codex/skills, ~/.claude/skills, ~/.cursor/skills) or project-level (<repo>/.codex/skills, etc.)
        #[arg(long, value_enum, default_value_t = SkillScopeArg::User)]
        scope: SkillScopeArg,
        /// Which agent(s) to install for
        #[arg(long, value_enum, default_value_t = SkillAgentArg::All)]
        agent: SkillAgentArg,
        /// Overwrite existing SKILL.md files
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Uninstall the embedded WorkMesh skill from agent skill directories
    Uninstall {
        /// Skill name (defaults to workmesh)
        #[arg(long)]
        name: Option<String>,
        /// Uninstall from user-level (~/.codex/skills, ~/.claude/skills, ~/.cursor/skills) or project-level (<repo>/.codex/skills, etc.)
        #[arg(long, value_enum, default_value_t = SkillScopeArg::User)]
        scope: SkillScopeArg,
        /// Which agent(s) to uninstall for
        #[arg(long, value_enum, default_value_t = SkillAgentArg::All)]
        agent: SkillAgentArg,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Install the embedded skill globally for detected agents under your home directory
    ///
    /// This only installs for agents that already have a home folder (e.g. ~/.codex, ~/.claude, ~/.cursor).
    InstallGlobal {
        /// Skill name (defaults to workmesh)
        #[arg(long)]
        name: Option<String>,
        /// Overwrite existing SKILL.md files
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Uninstall the embedded skill globally for detected agents under your home directory
    UninstallGlobal {
        /// Skill name (defaults to workmesh)
        #[arg(long)]
        name: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum FixCommand {
    /// List available fixers
    List {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Run all fixers (optionally scoped by --only/--exclude)
    All {
        /// Apply changes (default is check/dry-run)
        #[arg(long, action = ArgAction::SetTrue)]
        apply: bool,
        /// Explicitly run in check mode (default if --apply is not set)
        #[arg(long, action = ArgAction::SetTrue)]
        check: bool,
        /// Comma-separated list of fixers to include (uid,deps,ids)
        #[arg(long, value_delimiter = ',', value_enum)]
        only: Vec<FixTargetArg>,
        /// Comma-separated list of fixers to exclude (uid,deps,ids)
        #[arg(long, value_delimiter = ',', value_enum)]
        exclude: Vec<FixTargetArg>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Backfill missing task uid values
    Uid {
        /// Apply changes (default is check/dry-run)
        #[arg(long, action = ArgAction::SetTrue)]
        apply: bool,
        /// Explicitly run in check mode (default if --apply is not set)
        #[arg(long, action = ArgAction::SetTrue)]
        check: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Remove missing/duplicate dependencies from task dependency lists
    Deps {
        /// Apply changes (default is check/dry-run)
        #[arg(long, action = ArgAction::SetTrue)]
        apply: bool,
        /// Explicitly run in check mode (default if --apply is not set)
        #[arg(long, action = ArgAction::SetTrue)]
        check: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Detect/rekey duplicate task ids (same behavior as legacy `fix-ids`)
    Ids {
        /// Apply changes (default is check/dry-run)
        #[arg(long, action = ArgAction::SetTrue)]
        apply: bool,
        /// Explicitly run in check mode (default if --apply is not set)
        #[arg(long, action = ArgAction::SetTrue)]
        check: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
}

#[derive(Debug, Copy, Clone, ValueEnum, PartialEq, Eq, Hash)]
enum FixTargetArg {
    Uid,
    Deps,
    Ids,
}

#[derive(Debug, Copy, Clone, ValueEnum)]
enum SkillScopeArg {
    User,
    Project,
}

#[derive(Debug, Copy, Clone, ValueEnum)]
enum SkillAgentArg {
    Codex,
    Claude,
    Cursor,
    All,
}

#[derive(Debug, Copy, Clone, ValueEnum)]
enum SkillProfileArg {
    /// Hybrid router skill (auto-select CLI or MCP mode)
    Hybrid,
    /// CLI-only profile
    Cli,
    /// MCP-only profile
    Mcp,
    /// Install all profiles (hybrid + cli + mcp)
    All,
}

impl From<SkillScopeArg> for SkillScope {
    fn from(value: SkillScopeArg) -> Self {
        match value {
            SkillScopeArg::User => SkillScope::User,
            SkillScopeArg::Project => SkillScope::Project,
        }
    }
}

fn skill_names_for_profile(profile: SkillProfileArg) -> Vec<&'static str> {
    match profile {
        SkillProfileArg::Hybrid => vec!["workmesh"],
        SkillProfileArg::Cli => vec!["workmesh-cli"],
        SkillProfileArg::Mcp => vec!["workmesh-mcp"],
        SkillProfileArg::All => embedded_skill_ids(),
    }
}

fn merge_install_report(report: &mut SkillInstallReport, partial: SkillInstallReport) {
    report.written.extend(partial.written);
    report.skipped.extend(partial.skipped);
}

fn merge_uninstall_report(report: &mut SkillUninstallReport, partial: SkillUninstallReport) {
    report.removed.extend(partial.removed);
    report.missing.extend(partial.missing);
}

fn print_install_report(report: SkillInstallReport) {
    if report.written.is_empty() {
        println!("(no files written)");
    } else {
        for path in report.written {
            println!("{}", path.display());
        }
    }
    if !report.skipped.is_empty() {
        println!("(skipped existing; use --force to overwrite)");
        for path in report.skipped {
            println!("{}", path.display());
        }
    }
}

fn print_uninstall_report(report: SkillUninstallReport) {
    if report.removed.is_empty() {
        println!("(no files removed)");
    } else {
        for path in report.removed {
            println!("{}", path.display());
        }
    }
    if !report.missing.is_empty() {
        println!("(not found)");
        for path in report.missing {
            println!("{}", path.display());
        }
    }
}

#[derive(Debug, Clone)]
struct FixRunReport {
    fixer: String,
    detected: usize,
    fixed: usize,
    skipped: usize,
    warnings: Vec<String>,
    details: serde_json::Value,
}

fn fix_run_to_json(run: &FixRunReport) -> serde_json::Value {
    serde_json::json!({
        "fixer": run.fixer,
        "detected": run.detected,
        "fixed": run.fixed,
        "skipped": run.skipped,
        "warnings": run.warnings,
        "details": run.details,
    })
}

fn parse_fix_mode(apply: bool, check: bool) -> Result<bool> {
    if apply && check {
        die("choose either --apply or --check (or neither for default dry-run)");
    }
    Ok(apply)
}

fn all_fix_targets() -> Vec<FixTargetArg> {
    vec![FixTargetArg::Uid, FixTargetArg::Deps, FixTargetArg::Ids]
}

fn select_fix_targets(only: &[FixTargetArg], exclude: &[FixTargetArg]) -> Vec<FixTargetArg> {
    let mut selected = if only.is_empty() {
        all_fix_targets()
    } else {
        only.to_vec()
    };
    if !exclude.is_empty() {
        let exclude_set: HashSet<FixTargetArg> = exclude.iter().copied().collect();
        selected.retain(|item| !exclude_set.contains(item));
    }
    let mut seen = HashSet::new();
    selected.retain(|item| seen.insert(*item));
    selected
}

fn as_fixer_kind(target: FixTargetArg) -> FixerKind {
    match target {
        FixTargetArg::Uid => FixerKind::Uid,
        FixTargetArg::Deps => FixerKind::Deps,
        FixTargetArg::Ids => FixerKind::Ids,
    }
}

fn print_fix_report(report: &FixRunReport, apply: bool) {
    println!(
        "{} | detected={} {}={} skipped={}",
        report.fixer,
        report.detected,
        if apply { "fixed" } else { "would_fix" },
        if apply {
            report.fixed
        } else {
            report.detected.saturating_sub(report.skipped)
        },
        report.skipped
    );
    for warning in &report.warnings {
        println!("  warning: {}", warning);
    }
}

fn run_fix_target(backlog_dir: &Path, target: FixTargetArg, apply: bool) -> Result<FixRunReport> {
    let tasks = load_tasks(backlog_dir);
    match target {
        FixTargetArg::Uid => {
            let report = backfill_missing_uids(&tasks, apply)?;
            Ok(FixRunReport {
                fixer: FixerKind::Uid.as_str().to_string(),
                detected: report.detected,
                fixed: report.fixed,
                skipped: report.skipped,
                warnings: report.warnings,
                details: serde_json::json!(report.changes),
            })
        }
        FixTargetArg::Deps => {
            let report = fix_dependencies(&tasks, apply)?;
            Ok(FixRunReport {
                fixer: FixerKind::Deps.as_str().to_string(),
                detected: report.detected,
                fixed: report.fixed,
                skipped: report.skipped,
                warnings: report.warnings,
                details: serde_json::json!(report.changes),
            })
        }
        FixTargetArg::Ids => {
            let report = fix_duplicate_task_ids(backlog_dir, &tasks, FixIdsOptions { apply })?;
            Ok(FixRunReport {
                fixer: FixerKind::Ids.as_str().to_string(),
                detected: report.changes.len(),
                fixed: if apply { report.changes.len() } else { 0 },
                skipped: 0,
                warnings: report.warnings,
                details: serde_json::json!(report
                    .changes
                    .iter()
                    .map(|change| serde_json::json!({
                        "old_id": change.old_id,
                        "new_id": change.new_id,
                        "old_path": change.old_path,
                        "new_path": change.new_path,
                        "uid": change.uid,
                    }))
                    .collect::<Vec<_>>()),
            })
        }
    }
}

impl From<SkillAgentArg> for SkillAgent {
    fn from(value: SkillAgentArg) -> Self {
        match value {
            SkillAgentArg::Codex => SkillAgent::Codex,
            SkillAgentArg::Claude => SkillAgent::Claude,
            SkillAgentArg::Cursor => SkillAgent::Cursor,
            SkillAgentArg::All => SkillAgent::All,
        }
    }
}

#[derive(Subcommand)]
enum ContextCommand {
    /// Set context state for the current repo
    Set {
        #[arg(long)]
        project: Option<String>,
        /// Scope to an epic subtree
        #[arg(long)]
        epic: Option<String>,
        #[arg(long)]
        objective: Option<String>,
        /// Comma-separated task ids
        #[arg(long)]
        tasks: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Show the current context state
    Show {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Clear the current context state
    Clear {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum MigrateCommand {
    /// Detect legacy/deprecated structures and suggest migrations
    Audit {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Build an ordered migration plan from audit findings
    Plan {
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        include: Vec<String>,
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        exclude: Vec<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Apply migration plan (dry-run by default unless --apply is passed)
    Apply {
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        include: Vec<String>,
        #[arg(long, value_delimiter = ',', num_args = 0..)]
        exclude: Vec<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        apply: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        backup: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum SessionCommand {
    /// Save the current agent session to the global store (default: ~/.workmesh)
    Save {
        #[arg(long)]
        objective: String,
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        tasks: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// List recent agent sessions from the global store
    List {
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Show a specific agent session
    Show {
        session_id: String,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Resume from a session (prints summary + suggested next commands)
    Resume {
        /// Session id; if omitted, uses the current session pointer if present
        session_id: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Rebuild the global sessions index (JSONL) for fast queries
    IndexRebuild {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Refresh the global sessions index (JSONL)
    IndexRefresh {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    /// Verify the global sessions index (JSONL) against source events
    IndexVerify {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
        /// Do not update `updated_date` (default behavior touches on all mutations)
        #[arg(long, action = ArgAction::SetTrue)]
        no_touch: bool,
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
enum BoardByArg {
    Status,
    Phase,
    Priority,
}

impl BoardByArg {
    fn to_core(self) -> BoardBy {
        match self {
            BoardByArg::Status => BoardBy::Status,
            BoardByArg::Phase => BoardBy::Phase,
            BoardByArg::Priority => BoardBy::Priority,
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

fn is_done_status(status: &str) -> bool {
    status.eq_ignore_ascii_case("done")
}

fn effective_touch(touch: bool, no_touch: bool) -> bool {
    if no_touch {
        return false;
    }
    // Back-compat: `--touch` is still accepted, but touching is now the default on mutations.
    if touch {
        return true;
    }
    true
}

fn best_effort_git_snapshot(repo_root: &Path) -> GitSnapshot {
    let branch = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|value| !value.trim().is_empty());

    let head_sha = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|value| !value.trim().is_empty());

    let dirty = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("status")
        .arg("--porcelain")
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(!String::from_utf8_lossy(&out.stdout).trim().is_empty())
            } else {
                None
            }
        });

    GitSnapshot {
        branch,
        head_sha,
        dirty,
    }
}

fn best_effort_git_branch(repo_root: &Path) -> Option<String> {
    std::process::Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|value| !value.trim().is_empty())
}

fn render_session_line(session: &AgentSession) -> String {
    format!(
        "{} | {} | {} | {}",
        session.id, session.updated_at, session.cwd, session.objective
    )
}

fn render_session_detail(session: &AgentSession) -> String {
    let mut lines = Vec::new();
    lines.push(format!("id: {}", session.id));
    lines.push(format!("updated_at: {}", session.updated_at));
    lines.push(format!("cwd: {}", session.cwd));
    if let Some(repo_root) = session.repo_root.as_deref() {
        lines.push(format!("repo_root: {}", repo_root));
    }
    if let Some(project_id) = session.project_id.as_deref() {
        lines.push(format!("project_id: {}", project_id));
    }
    if let Some(epic_id) = session.epic_id.as_deref() {
        lines.push(format!("epic_id: {}", epic_id));
    }
    lines.push(format!("objective: {}", session.objective));
    if !session.working_set.is_empty() {
        lines.push(format!("working_set: {}", session.working_set.join(", ")));
    }
    if let Some(notes) = session.notes.as_deref() {
        if !notes.trim().is_empty() {
            lines.push(format!("notes: {}", notes));
        }
    }
    if let Some(git) = session.git.as_ref() {
        if git.branch.is_some() || git.head_sha.is_some() || git.dirty.is_some() {
            lines.push(format!(
                "git: branch={} sha={} dirty={}",
                git.branch.clone().unwrap_or_else(|| "-".to_string()),
                git.head_sha.clone().unwrap_or_else(|| "-".to_string()),
                git.dirty
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string())
            ));
        }
    }
    if let Some(checkpoint) = session.checkpoint.as_ref() {
        lines.push(format!("checkpoint: {}", checkpoint.path));
    }
    if let Some(handoff) = session.handoff.as_ref() {
        if !handoff.completed.is_empty() {
            lines.push(format!(
                "handoff.completed: {}",
                handoff.completed.join(" | ")
            ));
        }
        if !handoff.remaining.is_empty() {
            lines.push(format!(
                "handoff.remaining: {}",
                handoff.remaining.join(" | ")
            ));
        }
        if !handoff.decisions.is_empty() {
            lines.push(format!(
                "handoff.decisions: {}",
                handoff.decisions.join(" | ")
            ));
        }
        if !handoff.unknowns.is_empty() {
            lines.push(format!(
                "handoff.unknowns: {}",
                handoff.unknowns.join(" | ")
            ));
        }
        if let Some(next_step) = handoff.next_step.as_deref() {
            if !next_step.trim().is_empty() {
                lines.push(format!("handoff.next_step: {}", next_step));
            }
        }
    }
    lines.join("\n")
}

fn resume_script(session: &AgentSession) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("cd {}", session.cwd));

    if let Some(repo_root) = session.repo_root.as_deref() {
        lines.push(format!("workmesh --root {} context show", repo_root));
    }

    if let (Some(repo_root), Some(project_id)) =
        (session.repo_root.as_deref(), session.project_id.as_deref())
    {
        lines.push(format!(
            "workmesh --root {} resume --project {}",
            repo_root, project_id
        ));
        lines.push(format!("workmesh --root {} ready", repo_root));
    }

    if let Some(repo_root) = session.repo_root.as_deref() {
        if let Some(task_id) = session.working_set.first() {
            lines.push(format!("workmesh --root {} show {}", repo_root, task_id));
            lines.push(format!(
                "workmesh --root {} claim {} you",
                repo_root, task_id
            ));
        }
    }

    lines
}

fn auto_update_current_session(backlog_dir: &Path) -> Result<()> {
    let home = resolve_workmesh_home()?;
    let Some(current_id) = read_current_session_id(&home) else {
        return Ok(());
    };

    let sessions = load_sessions_latest_fast(&home)?;
    let Some(existing) = sessions.into_iter().find(|s| s.id == current_id) else {
        return Ok(());
    };

    let rr = repo_root_from_backlog(backlog_dir);
    let repo_root = rr.to_string_lossy().to_string();
    let repo_tasks = load_tasks(backlog_dir);
    let project_id = resolve_project_id(&rr, &repo_tasks, None);
    let epic_id = load_context_state(backlog_dir)
        .and_then(|c| c.scope.epic_id)
        .or_else(|| best_effort_git_branch(&rr).and_then(|b| extract_task_id_from_branch(&b)));

    let working_set: Vec<String> = repo_tasks
        .iter()
        .filter(|task| task.status.eq_ignore_ascii_case("in progress") || is_lease_active(task))
        .map(|task| task.id.clone())
        .collect();

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cwd_str = cwd.to_string_lossy().to_string();

    let mut checkpoint: Option<CheckpointRef> = None;
    let mut recent_changes: Option<workmesh_core::global_sessions::RecentChanges> = None;
    if let Ok(Some(summary)) = resume_summary(&rr, &project_id, None) {
        checkpoint = Some(CheckpointRef {
            path: summary.checkpoint_path.to_string_lossy().to_string(),
            timestamp: Some(summary.snapshot.generated_at.clone()),
        });
        recent_changes = Some(workmesh_core::global_sessions::RecentChanges {
            dirs: summary.snapshot.top_level_dirs.clone(),
            files: summary.snapshot.changed_files.clone(),
        });
    }

    let now = now_rfc3339();
    let updated = AgentSession {
        id: existing.id.clone(),
        created_at: existing.created_at.clone(),
        updated_at: now,
        cwd: cwd_str,
        repo_root: Some(repo_root),
        project_id: Some(project_id),
        epic_id: epic_id.or(existing.epic_id.clone()),
        objective: existing.objective.clone(),
        working_set,
        notes: existing.notes.clone(),
        git: Some(best_effort_git_snapshot(&rr)),
        checkpoint,
        recent_changes,
        handoff: existing.handoff.clone(),
    };

    append_session_saved(&home, updated.clone())?;
    set_current_session(&home, &updated.id)?;
    Ok(())
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

    if let Command::Install {
        skills,
        profile,
        scope,
        agent,
        force,
        json,
    } = &cli.command
    {
        if !skills {
            die("install currently supports only --skills");
        }
        let repo_root = repo_root_from_backlog(&cli.root);
        let mut report = SkillInstallReport::default();
        let names = skill_names_for_profile(*profile);
        for name in names.iter() {
            let partial = install_embedded_skill_report(
                Some(&repo_root),
                (*scope).into(),
                (*agent).into(),
                name,
                *force,
            )?;
            merge_install_report(&mut report, partial);
        }
        if *json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": true,
                    "profile": format!("{:?}", profile).to_lowercase(),
                    "scope": format!("{:?}", scope).to_lowercase(),
                    "agent": format!("{:?}", agent).to_lowercase(),
                    "skills": names,
                    "written": report.written,
                    "skipped": report.skipped
                }))?
            );
        } else {
            println!(
                "Installed profile={} scope={} agent={} skills={}",
                format!("{:?}", profile).to_lowercase(),
                format!("{:?}", scope).to_lowercase(),
                format!("{:?}", agent).to_lowercase(),
                names.join(", ")
            );
            print_install_report(report);
        }
        return Ok(());
    }

    if let Command::Uninstall {
        skills,
        profile,
        scope,
        agent,
        json,
    } = &cli.command
    {
        if !skills {
            die("uninstall currently supports only --skills");
        }
        let repo_root = repo_root_from_backlog(&cli.root);
        let mut report = SkillUninstallReport::default();
        let names = skill_names_for_profile(*profile);
        for name in names.iter() {
            let partial = uninstall_embedded_skill_report(
                Some(&repo_root),
                (*scope).into(),
                (*agent).into(),
                name,
            )?;
            merge_uninstall_report(&mut report, partial);
        }
        if *json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": true,
                    "profile": format!("{:?}", profile).to_lowercase(),
                    "scope": format!("{:?}", scope).to_lowercase(),
                    "agent": format!("{:?}", agent).to_lowercase(),
                    "skills": names,
                    "removed": report.removed,
                    "missing": report.missing
                }))?
            );
        } else {
            println!(
                "Uninstalled profile={} scope={} agent={} skills={}",
                format!("{:?}", profile).to_lowercase(),
                format!("{:?}", scope).to_lowercase(),
                format!("{:?}", agent).to_lowercase(),
                names.join(", ")
            );
            print_uninstall_report(report);
        }
        return Ok(());
    }

    if let Command::Doctor { json } = &cli.command {
        let report = doctor_report(&cli.root, "workmesh");
        if *json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            println!("root: {}", report["root"].as_str().unwrap_or(""));
            println!("repo_root: {}", report["repo_root"].as_str().unwrap_or(""));
            println!(
                "backlog_dir: {}",
                report["backlog_dir"].as_str().unwrap_or("")
            );
            println!("layout: {}", report["layout"].as_str().unwrap_or(""));
            if !report["context"].is_null() {
                let epic = report["context"]["scope"]["epic_id"].as_str().unwrap_or("");
                let project = report["context"]["project_id"].as_str().unwrap_or("");
                let mode = report["context"]["scope"]["mode"]
                    .as_str()
                    .unwrap_or("none");
                println!(
                    "context: project_id={} scope_mode={} epic_id={}",
                    project, mode, epic
                );
            } else {
                println!("context: (none)");
            }
            let present = report["index"]["present"].as_bool().unwrap_or(false);
            let entries = report["index"]["entries"].as_i64().unwrap_or(0);
            println!("index: present={} entries={}", present, entries);
            println!(
                "versions: workmesh={} workmesh-mcp={}",
                report["versions"]["workmesh"].as_str().unwrap_or(""),
                report["versions"]["workmesh_mcp"].as_str().unwrap_or("")
            );
        }
        return Ok(());
    }

    if let Command::Migrate { command, to, yes } = &cli.command {
        if let Some(migrate_cmd) = command {
            handle_migrate_workflow(&cli.root, migrate_cmd)?;
        } else {
            let resolution = resolve_backlog(&cli.root)?;
            let target = to.as_deref().unwrap_or("workmesh");
            handle_migrate_command(&resolution, target, *yes)?;
        }
        return Ok(());
    }

    let resolution = resolve_backlog(&cli.root)?;
    let backlog_dir = maybe_prompt_migration(&resolution)?;
    let tasks = load_tasks(&backlog_dir);
    let auto_checkpoint = auto_checkpoint_enabled(&cli);
    let auto_session = auto_session_enabled(&cli);

    match cli.command {
        Command::Board {
            all,
            by,
            focus,
            json,
        } => {
            let tasks = if all {
                load_tasks_with_archive(&backlog_dir)
            } else {
                load_tasks(&backlog_dir)
            };
            let context_state = if focus {
                load_context_state(&backlog_dir)
            } else {
                None
            };
            let scope_ids = context_state
                .as_ref()
                .and_then(|c| scope_ids_from_context(&tasks, c));
            let lanes = board_lanes(&tasks, by.to_core(), scope_ids.as_ref());

            if json {
                let payload: Vec<serde_json::Value> = lanes
                    .into_iter()
                    .map(|(key, lane_tasks)| {
                        let tasks_json: Vec<serde_json::Value> = lane_tasks
                            .into_iter()
                            .map(|t| task_to_json_value(t, false))
                            .collect();
                        serde_json::json!({
                            "lane": key,
                            "count": tasks_json.len(),
                            "tasks": tasks_json,
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }

            for (key, lane_tasks) in lanes {
                println!("## {} ({})", key, lane_tasks.len());
                for task in lane_tasks {
                    println!("{}", render_task_line(task));
                }
                println!();
            }
        }
        Command::Blockers { all, epic_id, json } => {
            let tasks = if all {
                load_tasks_with_archive(&backlog_dir)
            } else {
                load_tasks(&backlog_dir)
            };
            let context_state = load_context_state(&backlog_dir);
            let report =
                blockers_report_with_context(&tasks, context_state.as_ref(), epic_id.as_deref());

            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
                return Ok(());
            }

            println!("Scope: {}", report.scope);
            if !report.warnings.is_empty() {
                println!("Warnings:");
                for w in report.warnings.iter() {
                    println!("- {}", w);
                }
            }
            if report.blocked_tasks.is_empty() {
                println!("Blocked tasks: (none)");
            } else {
                println!("Blocked tasks:");
                for entry in report.blocked_tasks.iter() {
                    let mut parts = Vec::new();
                    if !entry.blockers.is_empty() {
                        parts.push(format!("blocked_by=[{}]", entry.blockers.join(", ")));
                    }
                    if !entry.missing_refs.is_empty() {
                        parts.push(format!("missing_refs=[{}]", entry.missing_refs.join(", ")));
                    }
                    println!(
                        "- {}: {} ({}) {}",
                        entry.id,
                        entry.title,
                        entry.status,
                        parts.join(" ")
                    );
                }
            }
            if report.top_blockers.is_empty() {
                println!("Top blockers: (none)");
            } else {
                println!("Top blockers:");
                for b in report.top_blockers.iter().take(10) {
                    println!("- {} blocks {}", b.id, b.blocked_count);
                }
            }
        }
        Command::List {
            all,
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
            let tasks = if all {
                load_tasks_with_archive(&backlog_dir)
            } else {
                load_tasks(&backlog_dir)
            };
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
            let context = load_context_state(&backlog_dir);
            let recommended = recommend_next_tasks_with_context(&tasks, context.as_ref());
            let task = recommended.first().map(|t| (*t).clone());
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
        Command::FixIds { apply, json } => {
            let report = fix_duplicate_task_ids(&backlog_dir, &tasks, FixIdsOptions { apply })?;
            if apply {
                audit_event(
                    &backlog_dir,
                    "fix_ids",
                    None,
                    serde_json::json!({ "changes": report.changes.len() }),
                )?;
                refresh_index_best_effort(&backlog_dir);
                maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
            }
            if json {
                let payload = serde_json::json!({
                    "ok": true,
                    "apply": apply,
                    "changes": report.changes.iter().map(|c| serde_json::json!({
                        "old_id": c.old_id,
                        "new_id": c.new_id,
                        "old_path": c.old_path,
                        "new_path": c.new_path,
                        "uid": c.uid,
                    })).collect::<Vec<_>>(),
                    "warnings": report.warnings,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else if report.changes.is_empty() {
                println!("No duplicate task ids found.");
            } else {
                for warning in &report.warnings {
                    eprintln!("warning: {}", warning);
                }
                for change in &report.changes {
                    println!("{} -> {}", change.old_id, change.new_id);
                }
                if !apply {
                    println!("Dry-run: re-run with --apply to write changes.");
                }
            }
        }
        Command::Fix { command } => match command {
            FixCommand::List { json } => {
                let fixers = all_fix_targets()
                    .into_iter()
                    .map(as_fixer_kind)
                    .map(|kind| kind.as_str().to_string())
                    .collect::<Vec<_>>();
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "fixers": fixers,
                        }))?
                    );
                } else {
                    for fixer in fixers {
                        println!("{}", fixer);
                    }
                }
            }
            FixCommand::All {
                apply,
                check,
                only,
                exclude,
                json,
            } => {
                let apply_mode = parse_fix_mode(apply, check)?;
                let targets = select_fix_targets(&only, &exclude);
                if targets.is_empty() {
                    die("No fixers selected. Adjust --only/--exclude.");
                }
                let mut runs = Vec::new();
                for target in targets {
                    runs.push(run_fix_target(&backlog_dir, target, apply_mode)?);
                }

                let total_detected: usize = runs.iter().map(|run| run.detected).sum();
                let total_fixed: usize = runs.iter().map(|run| run.fixed).sum();
                let total_skipped: usize = runs.iter().map(|run| run.skipped).sum();
                let runs_json: Vec<serde_json::Value> = runs.iter().map(fix_run_to_json).collect();

                if apply_mode {
                    audit_event(
                        &backlog_dir,
                        "fix_all",
                        None,
                        serde_json::json!({
                            "runs": runs_json.clone(),
                            "fixed": total_fixed,
                        }),
                    )?;
                    refresh_index_best_effort(&backlog_dir);
                    maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
                }

                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "ok": true,
                            "mode": if apply_mode { "apply" } else { "check" },
                            "runs": runs_json,
                            "totals": {
                                "detected": total_detected,
                                "fixed": total_fixed,
                                "skipped": total_skipped,
                            }
                        }))?
                    );
                } else {
                    for run in &runs {
                        print_fix_report(run, apply_mode);
                    }
                    if !apply_mode {
                        println!("Dry-run: re-run with --apply to write changes.");
                    }
                }
            }
            FixCommand::Uid { apply, check, json } => {
                let apply_mode = parse_fix_mode(apply, check)?;
                let run = run_fix_target(&backlog_dir, FixTargetArg::Uid, apply_mode)?;
                if apply_mode {
                    audit_event(
                        &backlog_dir,
                        "fix_uid",
                        None,
                        serde_json::json!({ "fixed": run.fixed }),
                    )?;
                    refresh_index_best_effort(&backlog_dir);
                    maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
                }
                if json {
                    let run_json = fix_run_to_json(&run);
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "ok": true,
                            "mode": if apply_mode { "apply" } else { "check" },
                            "run": run_json
                        }))?
                    );
                } else {
                    print_fix_report(&run, apply_mode);
                    if !apply_mode {
                        println!("Dry-run: re-run with --apply to write changes.");
                    }
                }
            }
            FixCommand::Deps { apply, check, json } => {
                let apply_mode = parse_fix_mode(apply, check)?;
                let run = run_fix_target(&backlog_dir, FixTargetArg::Deps, apply_mode)?;
                if apply_mode {
                    audit_event(
                        &backlog_dir,
                        "fix_deps",
                        None,
                        serde_json::json!({ "fixed": run.fixed }),
                    )?;
                    refresh_index_best_effort(&backlog_dir);
                    maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
                }
                if json {
                    let run_json = fix_run_to_json(&run);
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "ok": true,
                            "mode": if apply_mode { "apply" } else { "check" },
                            "run": run_json
                        }))?
                    );
                } else {
                    print_fix_report(&run, apply_mode);
                    if !apply_mode {
                        println!("Dry-run: re-run with --apply to write changes.");
                    }
                }
            }
            FixCommand::Ids { apply, check, json } => {
                let apply_mode = parse_fix_mode(apply, check)?;
                let run = run_fix_target(&backlog_dir, FixTargetArg::Ids, apply_mode)?;
                if apply_mode {
                    audit_event(
                        &backlog_dir,
                        "fix_ids",
                        None,
                        serde_json::json!({ "fixed": run.fixed }),
                    )?;
                    refresh_index_best_effort(&backlog_dir);
                    maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
                }
                if json {
                    let run_json = fix_run_to_json(&run);
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "ok": true,
                            "mode": if apply_mode { "apply" } else { "check" },
                            "run": run_json
                        }))?
                    );
                } else {
                    print_fix_report(&run, apply_mode);
                    if !apply_mode {
                        println!("Dry-run: re-run with --apply to write changes.");
                    }
                }
            }
        },
        Command::RekeyPrompt {
            all,
            include_body,
            limit,
            json,
        } => {
            let prompt = render_rekey_prompt(
                &backlog_dir,
                RekeyPromptOptions {
                    include_body,
                    include_archive: all,
                    limit,
                },
            );
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "ok": true,
                        "prompt": prompt,
                    }))?
                );
            } else {
                println!("{}", prompt);
            }
        }
        Command::RekeyApply {
            mapping,
            apply,
            all,
            strict,
            non_strict,
            json,
        } => {
            let mapping_text = read_content(None, mapping.as_deref())?;
            let mut request = parse_rekey_request(&mapping_text)?;
            if strict && non_strict {
                die("Invalid flags: use either --strict or --non-strict (or neither).");
            }
            if strict {
                request.strict = true;
            }
            if non_strict {
                request.strict = false;
            }
            let report = rekey_apply(
                &backlog_dir,
                &request,
                RekeyApplyOptions {
                    apply,
                    strict: request.strict,
                    include_archive: all,
                },
            )?;
            if apply {
                audit_event(
                    &backlog_dir,
                    "rekey_apply",
                    None,
                    serde_json::json!({ "changes": report.changes.len(), "strict": request.strict }),
                )?;
                refresh_index_best_effort(&backlog_dir);
                maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
            }
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::to_value(&report)?)?
                );
            } else if report.changes.is_empty() {
                println!("No tasks matched the mapping.");
            } else {
                for warning in &report.warnings {
                    eprintln!("warning: {}", warning);
                }
                for change in &report.changes {
                    if let Some(new_path) = &change.new_path {
                        println!(
                            "{} -> {} ({})",
                            change.old_id,
                            change.new_id,
                            new_path.display()
                        );
                    } else {
                        println!("{} -> {}", change.old_id, change.new_id);
                    }
                }
                if !apply {
                    println!("Dry-run: re-run with --apply to write changes.");
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
        Command::Session { command } => {
            let home = resolve_workmesh_home()?;
            match command {
                SessionCommand::Save {
                    objective,
                    cwd,
                    project,
                    tasks: task_list,
                    notes,
                    json,
                } => {
                    let cwd = cwd.unwrap_or(std::env::current_dir()?);
                    let cwd_str = cwd.to_string_lossy().to_string();

                    let tasks_override = task_list
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .map(split_csv);

                    let mut repo_root: Option<String> = None;
                    let mut project_id: Option<String> = project.clone();
                    let mut epic_id: Option<String> = None;
                    let mut working_set: Vec<String> = tasks_override.unwrap_or_default();
                    let mut git: Option<GitSnapshot> = None;
                    let mut checkpoint: Option<CheckpointRef> = None;
                    let mut recent_changes: Option<workmesh_core::global_sessions::RecentChanges> =
                        None;

                    if let Ok(backlog_dir) = locate_backlog_dir(&cwd) {
                        let rr = repo_root_from_backlog(&backlog_dir);
                        repo_root = Some(rr.to_string_lossy().to_string());
                        let repo_tasks = load_tasks(&backlog_dir);
                        epic_id = load_context_state(&backlog_dir)
                            .and_then(|c| c.scope.epic_id)
                            .or_else(|| {
                                best_effort_git_branch(&rr)
                                    .and_then(|b| extract_task_id_from_branch(&b))
                            });

                        if project_id.is_none() {
                            project_id =
                                Some(resolve_project_id(&rr, &repo_tasks, project.as_deref()));
                        }

                        if working_set.is_empty() {
                            working_set = repo_tasks
                                .iter()
                                .filter(|task| {
                                    task.status.eq_ignore_ascii_case("in progress")
                                        || is_lease_active(task)
                                })
                                .map(|task| task.id.clone())
                                .collect();
                        }

                        git = Some(best_effort_git_snapshot(&rr));
                        if let Some(pid) = project_id.as_deref() {
                            if let Ok(Some(summary)) = resume_summary(&rr, pid, None) {
                                checkpoint = Some(CheckpointRef {
                                    path: summary.checkpoint_path.to_string_lossy().to_string(),
                                    timestamp: Some(summary.snapshot.generated_at.clone()),
                                });
                                recent_changes =
                                    Some(workmesh_core::global_sessions::RecentChanges {
                                        dirs: summary.snapshot.top_level_dirs.clone(),
                                        files: summary.snapshot.changed_files.clone(),
                                    });
                            }
                        }
                    }

                    let now = now_rfc3339();
                    let session = AgentSession {
                        id: new_session_id(),
                        created_at: now.clone(),
                        updated_at: now,
                        cwd: cwd_str,
                        repo_root,
                        project_id,
                        epic_id,
                        objective,
                        working_set,
                        notes,
                        git,
                        checkpoint,
                        recent_changes,
                        handoff: None,
                    };

                    append_session_saved(&home, session.clone())?;
                    set_current_session(&home, &session.id)?;

                    if json {
                        println!("{}", serde_json::to_string_pretty(&session)?);
                    } else {
                        println!("Saved session {}", session.id);
                    }
                }
                SessionCommand::List { limit, json } => {
                    let mut sessions = load_sessions_latest_fast(&home)?;
                    if let Some(limit) = limit {
                        sessions.truncate(limit);
                    }
                    if json {
                        println!("{}", serde_json::to_string_pretty(&sessions)?);
                    } else if sessions.is_empty() {
                        println!("(no sessions)");
                    } else {
                        for session in sessions {
                            println!("{}", render_session_line(&session));
                        }
                    }
                }
                SessionCommand::Show { session_id, json } => {
                    let sessions = load_sessions_latest_fast(&home)?;
                    let session = sessions
                        .into_iter()
                        .find(|s| s.id == session_id)
                        .unwrap_or_else(|| die(&format!("Session not found: {}", session_id)));
                    if json {
                        println!("{}", serde_json::to_string_pretty(&session)?);
                    } else {
                        println!("{}", render_session_detail(&session));
                    }
                }
                SessionCommand::Resume { session_id, json } => {
                    let id = session_id
                        .or_else(|| read_current_session_id(&home))
                        .unwrap_or_else(|| {
                            die("No session id provided and no current session pointer found")
                        });
                    let sessions = load_sessions_latest_fast(&home)?;
                    let session = sessions
                        .into_iter()
                        .find(|s| s.id == id)
                        .unwrap_or_else(|| die(&format!("Session not found: {}", id)));
                    let script = resume_script(&session);
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(
                                &serde_json::json!({ "session": session, "resume_script": script })
                            )?
                        );
                    } else {
                        println!("{}", render_session_detail(&session));
                        println!();
                        println!("Suggested resume:");
                        for line in script {
                            println!("{}", line);
                        }
                    }
                }
                SessionCommand::IndexRebuild { json } => {
                    let summary = rebuild_sessions_index(&home)?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&summary)?);
                    } else {
                        println!("Indexed {} sessions -> {}", summary.indexed, summary.path);
                    }
                }
                SessionCommand::IndexRefresh { json } => {
                    let summary = refresh_sessions_index(&home)?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&summary)?);
                    } else {
                        println!("Indexed {} sessions -> {}", summary.indexed, summary.path);
                    }
                }
                SessionCommand::IndexVerify { json } => {
                    let report = verify_sessions_index(&home)?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    } else if report.ok {
                        println!("OK");
                    } else {
                        println!("{}", serde_json::to_string_pretty(&report)?);
                    }
                }
            }
        }
        Command::Context { command } => {
            let repo_root = repo_root_from_backlog(&backlog_dir);
            handle_context_command(&backlog_dir, &repo_root, command, false)?;
        }
        Command::Focus { command } => {
            let repo_root = repo_root_from_backlog(&backlog_dir);
            handle_context_command(&backlog_dir, &repo_root, command, true)?;
        }
        Command::Skill { command } => {
            let repo_root = repo_root_from_backlog(&backlog_dir);
            match command {
                SkillCommand::Show { name, json } => {
                    let skill_name = name
                        .as_deref()
                        .map(|value| value.trim())
                        .filter(|value| !value.is_empty())
                        .unwrap_or("workmesh");
                    let skill = load_skill_content(Some(&repo_root), skill_name)
                        .or_else(|| load_skill_content(None, skill_name));
                    let Some(skill) = skill else {
                        die(&format!("Skill not found: {}", skill_name));
                    };
                    if json {
                        println!("{}", serde_json::to_string_pretty(&skill)?);
                    } else {
                        println!("{}", skill.content);
                    }
                }
                SkillCommand::Install {
                    name,
                    scope,
                    agent,
                    force,
                    json,
                } => {
                    let skill_name = name
                        .as_deref()
                        .map(|value| value.trim())
                        .filter(|value| !value.is_empty())
                        .unwrap_or("workmesh");
                    let report = install_embedded_skill_report(
                        Some(&repo_root),
                        scope.into(),
                        agent.into(),
                        skill_name,
                        force,
                    )?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "ok": true,
                                "written": report.written,
                                "skipped": report.skipped
                            }))?
                        );
                    } else {
                        print_install_report(report);
                    }
                }
                SkillCommand::Uninstall {
                    name,
                    scope,
                    agent,
                    json,
                } => {
                    let skill_name = name
                        .as_deref()
                        .map(|value| value.trim())
                        .filter(|value| !value.is_empty())
                        .unwrap_or("workmesh");
                    let report = uninstall_embedded_skill_report(
                        Some(&repo_root),
                        scope.into(),
                        agent.into(),
                        skill_name,
                    )?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "ok": true,
                                "removed": report.removed,
                                "missing": report.missing
                            }))?
                        );
                    } else {
                        print_uninstall_report(report);
                    }
                }
                SkillCommand::InstallGlobal { name, force, json } => {
                    let skill_name = name
                        .as_deref()
                        .map(|value| value.trim())
                        .filter(|value| !value.is_empty())
                        .unwrap_or("workmesh");
                    let agents = detect_user_agents().unwrap_or_default();
                    let report = install_embedded_skill_global_auto_report(skill_name, force)?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "ok": true,
                                "detected_agents": agents,
                                "written": report.written,
                                "skipped": report.skipped
                            }))?
                        );
                    } else {
                        println!(
                            "Detected agents: {}",
                            agents
                                .iter()
                                .map(|a| format!("{:?}", a))
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        print_install_report(report);
                    }
                }
                SkillCommand::UninstallGlobal { name, json } => {
                    let skill_name = name
                        .as_deref()
                        .map(|value| value.trim())
                        .filter(|value| !value.is_empty())
                        .unwrap_or("workmesh");
                    let agents = detect_user_agents().unwrap_or_default();
                    let report = uninstall_embedded_skill_global_auto_report(skill_name)?;
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "ok": true,
                                "detected_agents": agents,
                                "removed": report.removed,
                                "missing": report.missing
                            }))?
                        );
                    } else {
                        println!(
                            "Detected agents: {}",
                            agents
                                .iter()
                                .map(|a| format!("{:?}", a))
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        print_uninstall_report(report);
                    }
                }
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
            no_touch,
        } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            if is_done_status(&status) {
                if let Err(err) = ensure_can_mark_done(&tasks, task) {
                    die(&err);
                }
            }
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let touch = effective_touch(touch, no_touch);
            update_task_field(path, "status", Some(status.clone().into()))?;
            if touch || is_done_status(&status) {
                update_task_field(path, "updated_date", Some(now_timestamp().into()))?;
            }
            audit_event(
                &backlog_dir,
                "set_status",
                Some(&task.id),
                serde_json::json!({ "status": status.clone() }),
            )?;
            refresh_index_best_effort(&backlog_dir);
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
            println!("Updated {} status -> {}", task.id, status);
        }
        Command::Claim {
            task_id,
            owner,
            minutes,
            touch,
            no_touch,
        } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let touch = effective_touch(touch, no_touch);
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
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
            println!("Claimed {} lease -> {}", task.id, lease.owner);
        }
        Command::Release {
            task_id,
            touch,
            no_touch,
        } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let touch = effective_touch(touch, no_touch);
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
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
            println!("Released {} lease", task.id);
        }
        Command::Bulk { command } => match command {
            BulkCommand::SetStatus {
                tasks: task_ids,
                status,
                touch,
                no_touch,
                json,
            } => handle_bulk_set_status(
                &backlog_dir,
                &tasks,
                task_ids,
                status,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?,
            BulkCommand::SetField {
                tasks: task_ids,
                field,
                value,
                touch,
                no_touch,
                json,
            } => handle_bulk_set_field(
                &backlog_dir,
                &tasks,
                task_ids,
                field,
                value,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?,
            BulkCommand::LabelAdd {
                tasks: task_ids,
                label,
                touch,
                no_touch,
                json,
            } => handle_bulk_label_add(
                &backlog_dir,
                &tasks,
                task_ids,
                label,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?,
            BulkCommand::LabelRemove {
                tasks: task_ids,
                label,
                touch,
                no_touch,
                json,
            } => handle_bulk_label_remove(
                &backlog_dir,
                &tasks,
                task_ids,
                label,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?,
            BulkCommand::DepAdd {
                tasks: task_ids,
                dependency,
                touch,
                no_touch,
                json,
            } => handle_bulk_dep_add(
                &backlog_dir,
                &tasks,
                task_ids,
                dependency,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?,
            BulkCommand::DepRemove {
                tasks: task_ids,
                dependency,
                touch,
                no_touch,
                json,
            } => handle_bulk_dep_remove(
                &backlog_dir,
                &tasks,
                task_ids,
                dependency,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?,
            BulkCommand::Note {
                tasks: task_ids,
                note,
                section,
                touch,
                no_touch,
                json,
            } => handle_bulk_note(
                &backlog_dir,
                &tasks,
                task_ids,
                note,
                section,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?,
        },
        Command::BulkSetStatus {
            tasks: task_ids,
            status,
            touch,
            no_touch,
            json,
        } => {
            handle_bulk_set_status(
                &backlog_dir,
                &tasks,
                task_ids,
                status,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?;
        }
        Command::BulkSetField {
            tasks: task_ids,
            field,
            value,
            touch,
            no_touch,
            json,
        } => {
            handle_bulk_set_field(
                &backlog_dir,
                &tasks,
                task_ids,
                field,
                value,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?;
        }
        Command::BulkLabelAdd {
            tasks: task_ids,
            label,
            touch,
            no_touch,
            json,
        } => {
            handle_bulk_label_add(
                &backlog_dir,
                &tasks,
                task_ids,
                label,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?;
        }
        Command::BulkLabelRemove {
            tasks: task_ids,
            label,
            touch,
            no_touch,
            json,
        } => {
            handle_bulk_label_remove(
                &backlog_dir,
                &tasks,
                task_ids,
                label,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?;
        }
        Command::BulkDepAdd {
            tasks: task_ids,
            dependency,
            touch,
            no_touch,
            json,
        } => {
            handle_bulk_dep_add(
                &backlog_dir,
                &tasks,
                task_ids,
                dependency,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?;
        }
        Command::BulkDepRemove {
            tasks: task_ids,
            dependency,
            touch,
            no_touch,
            json,
        } => {
            handle_bulk_dep_remove(
                &backlog_dir,
                &tasks,
                task_ids,
                dependency,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?;
        }
        Command::BulkNote {
            tasks: task_ids,
            note,
            section,
            touch,
            no_touch,
            json,
        } => {
            handle_bulk_note(
                &backlog_dir,
                &tasks,
                task_ids,
                note,
                section,
                effective_touch(touch, no_touch),
                json,
                auto_checkpoint,
                auto_session,
            )?;
        }
        Command::SetField {
            task_id,
            field,
            value,
            touch,
            no_touch,
        } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let touch = effective_touch(touch, no_touch);
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
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
            println!("Updated {} {} -> {}", task.id, field, value);
        }
        Command::LabelAdd {
            task_id,
            label,
            touch,
            no_touch,
        } => {
            update_list_field(
                &backlog_dir,
                &tasks,
                &task_id,
                "labels",
                &label,
                true,
                effective_touch(touch, no_touch),
            )?;
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
        }
        Command::LabelRemove {
            task_id,
            label,
            touch,
            no_touch,
        } => {
            update_list_field(
                &backlog_dir,
                &tasks,
                &task_id,
                "labels",
                &label,
                false,
                effective_touch(touch, no_touch),
            )?;
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
        }
        Command::DepAdd {
            task_id,
            dependency,
            touch,
            no_touch,
        } => {
            update_list_field(
                &backlog_dir,
                &tasks,
                &task_id,
                "dependencies",
                &dependency,
                true,
                effective_touch(touch, no_touch),
            )?;
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
        }
        Command::DepRemove {
            task_id,
            dependency,
            touch,
            no_touch,
        } => {
            update_list_field(
                &backlog_dir,
                &tasks,
                &task_id,
                "dependencies",
                &dependency,
                false,
                effective_touch(touch, no_touch),
            )?;
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
        }
        Command::Note {
            task_id,
            note,
            section,
            touch,
            no_touch,
        } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let touch = effective_touch(touch, no_touch);
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
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
            println!("Added note to {}", task.id);
        }
        Command::SetBody {
            task_id,
            text,
            file,
            touch,
            no_touch,
        } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let touch = effective_touch(touch, no_touch);
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
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
            println!("Updated body for {}", task.id);
        }
        Command::SetSection {
            task_id,
            section,
            text,
            file,
            touch,
            no_touch,
        } => {
            let task = find_task(&tasks, &task_id).unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let path = task.file_path.as_ref().unwrap_or_else(|| {
                die(&format!("Task not found: {}", task_id));
            });
            let touch = effective_touch(touch, no_touch);
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
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
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
            let task_id = match id {
                Some(value) => value,
                None => {
                    let repo_root = repo_root_from_backlog(&backlog_dir);
                    let branch = core_git_branch(&repo_root).unwrap_or_else(|| "work".to_string());
                    let initiative = ensure_branch_initiative(&repo_root, &branch)?;
                    next_namespaced_task_id(&tasks, &initiative)
                }
            };
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
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
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
            let task_id = match id {
                Some(value) => value,
                None => {
                    let repo_root = repo_root_from_backlog(&backlog_dir);
                    let branch = core_git_branch(&repo_root).unwrap_or_else(|| "work".to_string());
                    let initiative = ensure_branch_initiative(&repo_root, &branch)?;
                    next_namespaced_task_id(&tasks, &initiative)
                }
            };
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
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
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
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
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
                Some(cmd) => {
                    // `shell_words` is Unix-shell oriented and treats backslashes as escapes,
                    // which breaks Windows strings like `cmd /C C:\path\plantuml.cmd`.
                    // On Windows, keep parsing simple and predictable: whitespace-split.
                    if cfg!(windows) {
                        Some(
                            cmd.split_whitespace()
                                .map(|part| part.to_string())
                                .collect(),
                        )
                    } else {
                        Some(shell_words::split(&cmd).map_err(anyhow::Error::msg)?)
                    }
                }
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
        Command::Install { .. } => {
            unreachable!("install handled before backlog resolution");
        }
        Command::Uninstall { .. } => {
            unreachable!("uninstall handled before backlog resolution");
        }
        Command::Doctor { .. } => {
            unreachable!("doctor handled before backlog resolution");
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
            maybe_auto_checkpoint(&backlog_dir, auto_checkpoint, auto_session);
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

fn prompts_disabled() -> bool {
    // CI-friendly: some environments report stdin as a terminal but still cannot read input.
    // This opt-out keeps commands non-interactive and deterministic.
    std::env::var("WORKMESH_NO_PROMPT")
        .ok()
        .map(|value| {
            let v = value.trim().to_lowercase();
            v == "1" || v == "true" || v == "yes" || v == "y"
        })
        .unwrap_or(false)
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
    if prompts_disabled() || !io::stdin().is_terminal() {
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
    if !yes && io::stdin().is_terminal() && !prompts_disabled() {
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

fn handle_context_command(
    backlog_dir: &Path,
    repo_root: &Path,
    command: ContextCommand,
    focus_alias: bool,
) -> Result<()> {
    let state_key = if focus_alias { "focus" } else { "context" };
    let command_label = if focus_alias { "Focus" } else { "Context" };
    let clear_action = if focus_alias {
        "focus_clear"
    } else {
        "context_clear"
    };
    let set_action = if focus_alias {
        "focus_set"
    } else {
        "context_set"
    };

    match command {
        ContextCommand::Set {
            project,
            epic,
            objective,
            tasks: task_list,
            json,
        } => {
            let inferred_project = infer_project_id(repo_root);
            let inferred_epic_id = match epic {
                Some(value) => Some(value),
                None => {
                    best_effort_git_branch(repo_root).and_then(|b| extract_task_id_from_branch(&b))
                }
            };
            let task_ids = task_list
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(split_csv)
                .unwrap_or_default();
            let scope = if inferred_epic_id
                .as_deref()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
            {
                ContextScope {
                    mode: ContextScopeMode::Epic,
                    epic_id: inferred_epic_id.clone(),
                    task_ids: Vec::new(),
                }
            } else if !task_ids.is_empty() {
                ContextScope {
                    mode: ContextScopeMode::Tasks,
                    epic_id: None,
                    task_ids: task_ids.clone(),
                }
            } else {
                ContextScope {
                    mode: ContextScopeMode::None,
                    epic_id: None,
                    task_ids: Vec::new(),
                }
            };

            let state = ContextState {
                version: 1,
                project_id: project.or(inferred_project),
                objective,
                scope,
                updated_at: None,
            };
            let path = save_context(backlog_dir, state.clone())?;
            audit_event(
                backlog_dir,
                set_action,
                state.scope.epic_id.as_deref(),
                serde_json::json!({
                    "project_id": state.project_id.clone(),
                    "objective": state.objective.clone(),
                    "scope": state.scope.clone()
                }),
            )?;
            if json {
                let mut payload = serde_json::json!({
                    "ok": true,
                    "path": path
                });
                payload[state_key] = if focus_alias {
                    legacy_focus_payload(&state)
                } else {
                    serde_json::to_value(&state)?
                };
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("{} saved: {}", command_label, path.display());
            }
        }
        ContextCommand::Show { json } => {
            let context = infer_context_state(repo_root, backlog_dir);
            if json {
                let mut payload = serde_json::json!({
                    "path": context_path(backlog_dir)
                });
                payload[state_key] = if focus_alias {
                    match context.as_ref() {
                        Some(state) => legacy_focus_payload(state),
                        None => serde_json::Value::Null,
                    }
                } else {
                    serde_json::to_value(&context)?
                };
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else if let Some(context) = context {
                println!(
                    "project_id: {}",
                    context.project_id.unwrap_or_else(|| "(none)".into())
                );
                println!(
                    "objective: {}",
                    context.objective.unwrap_or_else(|| "(none)".into())
                );
                println!("scope.mode: {:?}", context.scope.mode);
                if let Some(epic_id) = context.scope.epic_id.as_deref() {
                    println!("scope.epic_id: {}", epic_id);
                }
                if !context.scope.task_ids.is_empty() {
                    println!("scope.task_ids: {}", context.scope.task_ids.join(", "));
                }
                println!();
                println!("Next:");
                println!("- workmesh --root . ready --json");
                println!("- workmesh --root . claim <task-id> <owner> --minutes 60");
            } else {
                println!("(no {} set)", state_key);
            }
        }
        ContextCommand::Clear { json } => {
            let cleared = clear_context(backlog_dir)?;
            if cleared {
                audit_event(backlog_dir, clear_action, None, serde_json::json!({}))?;
            }
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "ok": true,
                        "cleared": cleared
                    }))?
                );
            } else if cleared {
                println!("{} cleared", command_label);
            } else {
                println!("(no {} to clear)", state_key);
            }
        }
    }

    Ok(())
}

fn legacy_focus_payload(state: &ContextState) -> serde_json::Value {
    let (epic_id, working_set) = match state.scope.mode {
        ContextScopeMode::Epic => (state.scope.epic_id.clone(), Vec::new()),
        ContextScopeMode::Tasks => (None, state.scope.task_ids.clone()),
        ContextScopeMode::None => (None, Vec::new()),
    };
    serde_json::json!({
        "project_id": state.project_id.clone(),
        "epic_id": epic_id,
        "objective": state.objective.clone(),
        "working_set": working_set
    })
}

fn handle_migrate_workflow(root: &Path, command: &MigrateCommand) -> Result<()> {
    match command {
        MigrateCommand::Audit { json } => {
            let report = audit_deprecations(root)?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if report.findings.is_empty() {
                println!("No legacy/deprecated structures detected.");
            } else {
                println!("Detected {} finding(s):", report.findings.len());
                for finding in report.findings {
                    println!(
                        "- [{}] {} ({})",
                        finding.severity, finding.title, finding.id
                    );
                    if let Some(action) = finding.suggested_action {
                        println!("  suggested_action: {}", action);
                    }
                }
            }
        }
        MigrateCommand::Plan {
            include,
            exclude,
            json,
        } => {
            let report = audit_deprecations(root)?;
            let plan = plan_migrations(
                &report,
                &MigrationPlanOptions {
                    include: include.clone(),
                    exclude: exclude.clone(),
                },
            );
            if *json {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            } else if plan.steps.is_empty() {
                println!("No migration steps required.");
            } else {
                println!("Migration plan:");
                for (idx, step) in plan.steps.iter().enumerate() {
                    println!(
                        "{}. {} [{}] - {}",
                        idx + 1,
                        step.action,
                        if step.required {
                            "required"
                        } else {
                            "recommended"
                        },
                        step.reason
                    );
                }
                if !plan.warnings.is_empty() {
                    println!("Warnings:");
                    for warning in plan.warnings {
                        println!("- {}", warning);
                    }
                }
            }
        }
        MigrateCommand::Apply {
            include,
            exclude,
            apply,
            backup,
            json,
        } => {
            let report = audit_deprecations(root)?;
            let plan = plan_migrations(
                &report,
                &MigrationPlanOptions {
                    include: include.clone(),
                    exclude: exclude.clone(),
                },
            );
            let result = apply_migration_plan(
                root,
                &plan,
                &MigrationApplyOptions {
                    dry_run: !*apply,
                    backup: *backup,
                },
            )?;
            if *json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                if !*apply {
                    println!("Dry-run migration complete (no files changed).");
                    println!("Use `workmesh --root . migrate apply --apply` to write changes.");
                }
                println!("Applied: {}", result.applied.len());
                for step in result.applied {
                    println!("- {}", step);
                }
                if !result.skipped.is_empty() {
                    println!("Skipped:");
                    for step in result.skipped {
                        println!("- {}", step);
                    }
                }
                if !result.backups.is_empty() {
                    println!("Backups:");
                    for path in result.backups {
                        println!("- {}", path);
                    }
                }
                if !result.warnings.is_empty() {
                    println!("Warnings:");
                    for warning in result.warnings {
                        println!("- {}", warning);
                    }
                }
            }
        }
    }
    Ok(())
}

fn load_context_state(backlog_dir: &Path) -> Option<ContextState> {
    if let Ok(Some(context)) = load_context(backlog_dir) {
        return Some(context);
    }
    // Legacy fallback for repos not yet migrated.
    let legacy = load_focus(backlog_dir).ok().flatten()?;
    Some(workmesh_core::context::context_from_legacy_focus(
        legacy.project_id,
        legacy.epic_id,
        legacy.objective,
        legacy.working_set,
    ))
}

fn infer_context_state(repo_root: &Path, backlog_dir: &Path) -> Option<ContextState> {
    if let Some(existing) = load_context_state(backlog_dir) {
        return Some(existing);
    }
    let inferred_project = infer_project_id(repo_root);
    let inferred_epic =
        best_effort_git_branch(repo_root).and_then(|b| extract_task_id_from_branch(&b));
    if inferred_project.is_none() && inferred_epic.is_none() {
        return None;
    }
    let mut scope = ContextScope {
        mode: ContextScopeMode::None,
        epic_id: None,
        task_ids: Vec::new(),
    };
    if let Some(epic_id) = inferred_epic {
        scope.mode = ContextScopeMode::Epic;
        scope.epic_id = Some(epic_id);
    }
    Some(ContextState {
        version: 1,
        project_id: inferred_project,
        objective: None,
        scope,
        updated_at: None,
    })
}

fn handle_bulk_set_status(
    backlog_dir: &Path,
    tasks: &[Task],
    task_ids: Vec<String>,
    status: String,
    touch: bool,
    json: bool,
    auto_checkpoint: bool,
    auto_session: bool,
) -> Result<()> {
    let ids = normalize_task_ids(split_list(&task_ids));
    if ids.is_empty() {
        die("No tasks provided");
    }
    let (selected, missing) = select_tasks_with_missing(tasks, &ids);
    let mut updated = Vec::new();
    for task in selected {
        if is_done_status(&status) {
            if let Err(err) = ensure_can_mark_done(tasks, task) {
                die(&err);
            }
        }
        let path = task.file_path.as_ref().unwrap_or_else(|| {
            die(&format!("Task not found: {}", task.id));
        });
        update_task_field(path, "status", Some(FieldValue::Scalar(status.clone())))?;
        if touch || is_done_status(&status) {
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
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint, auto_session);
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
    auto_session: bool,
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
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint, auto_session);
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
    auto_session: bool,
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
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint, auto_session);
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
    auto_session: bool,
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
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint, auto_session);
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
    auto_session: bool,
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
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint, auto_session);
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
    auto_session: bool,
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
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint, auto_session);
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
    auto_session: bool,
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
    maybe_auto_checkpoint(backlog_dir, auto_checkpoint, auto_session);
    emit_bulk_result(&updated, &missing, json);
    Ok(())
}

fn best_practices_text() -> &'static str {
    "workmesh best practices\n\nDependencies:\n- Add dependencies whenever a task is blocked by other work.\n- Prefer explicit task ids (task-042) over vague references.\n- Update dependencies as status changes to avoid stale blockers.\n- Use validate to catch missing or broken dependency chains.\n\nDerived files:\n- Ignore derived artifacts like `workmesh/.index/` and `workmesh/.audit.log` in git.\n- If they show up as changes, rebuild/refresh and do not commit them.\n\nLabels:\n- Use labels to group work (docs, infra, ops).\n- Keep labels short and consistent.\n\nNotes:\n- Capture blockers or decisions in notes for future context.\n"
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

fn auto_session_enabled(cli: &Cli) -> bool {
    if cli.auto_session_save {
        return true;
    }
    env_flag_true("WORKMESH_AUTO_SESSION")
}

fn env_flag_true(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_lowercase())
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn maybe_auto_checkpoint(backlog_dir: &Path, auto_checkpoint: bool, auto_session: bool) {
    if auto_checkpoint {
        let tasks = load_tasks(backlog_dir);
        let options = CheckpointOptions {
            project_id: None,
            checkpoint_id: None,
            audit_limit: 10,
        };
        let _ = write_checkpoint(backlog_dir, &tasks, &options);
    }

    if auto_session {
        let _ = auto_update_current_session(backlog_dir);
    }
}

fn refresh_index_best_effort(backlog_dir: &Path) {
    let _ = refresh_index(backlog_dir);
}

fn die(message: &str) -> ! {
    eprintln!("{}", message);
    std::process::exit(1);
}
