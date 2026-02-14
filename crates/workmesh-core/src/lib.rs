//! Core domain types for WorkMesh.

pub mod archive;
pub mod audit;
pub mod backlog;
pub mod bootstrap;
pub mod config;
pub mod context;
pub mod doctor;
pub mod fix;
pub mod focus;
pub mod gantt;
pub mod global_sessions;
pub mod id_fix;
pub mod index;
pub mod initiative;
pub mod migration;
pub mod migration_audit;
pub mod project;
pub mod quickstart;
pub mod rekey;
pub mod session;
pub mod skills;
pub mod task;
pub mod task_ops;
pub mod truth;
pub mod views;
pub mod worktrees;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
pub mod test_env {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn global_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    pub fn lock() -> MutexGuard<'static, ()> {
        global_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn version_is_not_empty() {
        assert!(!version().is_empty());
    }
}
