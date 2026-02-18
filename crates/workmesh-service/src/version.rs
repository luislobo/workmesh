pub const FULL: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "+git.",
    env!("WORKMESH_GIT_COUNT"),
    ".",
    env!("WORKMESH_GIT_SHA"),
    env!("WORKMESH_GIT_DIRTY")
);

#[cfg(test)]
mod tests {
    use super::FULL;

    #[test]
    fn version_includes_current_git_sha() {
        let output = std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .expect("git rev-parse");
        assert!(output.status.success());
        let sha = String::from_utf8(output.stdout).expect("utf8");
        let sha = sha.trim();
        assert!(
            FULL.contains(sha),
            "version string does not include git sha; version={FULL} sha={sha}"
        );
    }
}
