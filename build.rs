//! Build script: captures the git commit SHA at compile time.
//! Exposes it as `env!("GIT_SHA")` in application code.
//!
//! Resolution order:
//! 1. `GIT_SHA` environment variable (set by Docker build arg in CI)
//! 2. `git rev-parse --short HEAD` (local dev builds)
//! 3. Falls back to "unknown"

use std::process::Command;

fn main() {
    // Prefer the GIT_SHA env var (set by CI / Dockerfile ARG)
    let sha = std::env::var("GIT_SHA")
        .ok()
        .filter(|s| !s.is_empty() && s != "unknown")
        .unwrap_or_else(|| {
            // Fall back to git rev-parse for local builds
            Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| {
                    let s = String::from_utf8(o.stdout).ok()?;
                    let trimmed = s.trim().to_string();
                    if trimmed.is_empty() { None } else { Some(trimmed) }
                })
                .unwrap_or_else(|| "unknown".to_string())
        });

    println!("cargo:rustc-env=GIT_SHA={sha}");

    // Re-run if HEAD changes (new commit) — only relevant for local builds
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-env-changed=GIT_SHA");
}
