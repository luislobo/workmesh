//! Core domain types for WorkMesh.

pub mod archive;
pub mod audit;
pub mod backlog;
pub mod config;
pub mod doctor;
pub mod focus;
pub mod gantt;
pub mod global_sessions;
pub mod index;
pub mod id_fix;
pub mod initiative;
pub mod rekey;
pub mod migration;
pub mod project;
pub mod quickstart;
pub mod session;
pub mod skills;
pub mod task;
pub mod task_ops;
pub mod views;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn version_is_not_empty() {
        assert!(!version().is_empty());
    }
}
