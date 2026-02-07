use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(text.trim().to_string())
}

fn rerun_if_changed(path: &str) {
    // cargo:rerun-if-changed paths are interpreted relative to the crate manifest dir.
    // Git paths are relative to the repo root, so we normalize them to absolute paths.
    let Some(toplevel) = git(&["rev-parse", "--show-toplevel"]) else {
        return;
    };
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        println!("cargo:rerun-if-changed={}", p.display());
    } else {
        println!(
            "cargo:rerun-if-changed={}",
            std::path::Path::new(&toplevel).join(p).display()
        );
    }
}

fn main() {
    // Windows CI was hitting a runtime stack overflow very early in CLI startup.
    // The root cause is likely deep call stacks in clap/formatting; bumping the
    // default stack size avoids spurious crashes and keeps behavior consistent
    // across platforms.
    if std::env::var("CARGO_CFG_TARGET_FAMILY").ok().as_deref() == Some("windows") {
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        if target_env == "msvc" {
            // MSVC linker flag (bytes).
            println!("cargo:rustc-link-arg=/STACK:8388608");
        } else {
            // MinGW/ld flag (bytes).
            println!("cargo:rustc-link-arg=-Wl,--stack,8388608");
        }
    }

    // Recompute when git state changes. Paths are repo-relative; normalize for Cargo.
    if let Some(head) = git(&["rev-parse", "--git-path", "HEAD"]) {
        rerun_if_changed(&head);
    }
    if let Some(index) = git(&["rev-parse", "--git-path", "index"]) {
        rerun_if_changed(&index);
    }

    let sha = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "nogit".to_string());
    let count = git(&["rev-list", "--count", "HEAD"]).unwrap_or_else(|| "0".to_string());
    // Treat any staged/unstaged/untracked change as dirty.
    let dirty = match git(&["status", "--porcelain"]) {
        Some(s) if s.is_empty() => "",
        _ => ".dirty",
    };

    println!("cargo:rustc-env=WORKMESH_GIT_SHA={}", sha);
    println!("cargo:rustc-env=WORKMESH_GIT_COUNT={}", count);
    println!("cargo:rustc-env=WORKMESH_GIT_DIRTY={}", dirty);
}
