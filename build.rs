//! Build script: captures the git commit SHA at compile time.
//! Exposes it as `env!("GIT_SHA")` in application code.

use std::process::Command;

fn main() {
    // Get the short git SHA; fall back to "unknown" if not in a git repo
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();

    let sha = match output {
        Ok(o) if o.status.success() => {
            String::from_utf8(o.stdout)
                .unwrap_or_default()
                .trim()
                .to_string()
        }
        _ => "unknown".to_string(),
    };

    println!("cargo:rustc-env=GIT_SHA={sha}");

    // Re-run if HEAD changes (new commit)
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
}
