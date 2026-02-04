//! Core domain types for WorkMesh.

pub mod backlog;
pub mod audit;
pub mod project;
pub mod index;
pub mod task;
pub mod task_ops;
pub mod gantt;

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
