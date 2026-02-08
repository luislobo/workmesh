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
    let p = std::path::Path::new(path);
    // `git rev-parse --git-path <x>` returns a path relative to the *current working directory*,
    // which for Cargo build scripts is typically the crate dir (not the repo root).
    //
    // Cargo expects rerun-if-changed paths relative to the crate dir as well, but we emit absolute
    // paths to avoid any ambiguity.
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        // If current_dir fails, fall back to the raw path (still works relative to the crate dir).
        std::env::current_dir().map(|cwd| cwd.join(p)).unwrap_or_else(|_| p.to_path_buf())
    };
    println!("cargo:rerun-if-changed={}", abs.display());
}

fn main() {
    // Recompute when git state changes. Paths are repo-relative; normalize for Cargo.
    if let Some(head) = git(&["rev-parse", "--git-path", "HEAD"]) {
        rerun_if_changed(&head);
    }
    // HEAD often points at a symbolic ref (e.g. refs/heads/main). That ref file changes on commit,
    // while .git/HEAD typically does not, so we need to watch it as well.
    if let Some(head_ref) = git(&["symbolic-ref", "-q", "HEAD"]) {
        if let Some(head_ref_path) = git(&["rev-parse", "--git-path", &head_ref]) {
            rerun_if_changed(&head_ref_path);
        }
    }
    // Some repos store refs in packed-refs instead of loose ref files.
    if let Some(packed_refs) = git(&["rev-parse", "--git-path", "packed-refs"]) {
        rerun_if_changed(&packed_refs);
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
