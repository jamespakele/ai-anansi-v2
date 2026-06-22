/// Inbox watcher — polls a directory and runs the full skill pipeline on each file.
///
/// Prompts are assembled at runtime from the live skill files in `skills_dir`, so
/// any edits you make in claude-cowork are picked up automatically without a rebuild.
///
/// Archive layout per run:
///   /data/archive/<slug>-<YYYYMMDD-HHmmss>/
///     source.md                   ← moved from inbox
///     projects-areas-toc.md       ← Stage 1a checkpoint
///     projects-areas-typed.md     ← Stage 1a checkpoint
///     resources-toc.md            ← Stage 1b checkpoint
///     resources-typed.md          ← Stage 1b checkpoint
///     <slug>-atomized.md          ← Stage 2 output
///     <slug>-toc.md               ← Stage 2 manifest
///     pipeline.log                ← newline-delimited JSON event log
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::time::sleep;

use crate::config::Config;
use crate::db::DbPool;
use crate::llm::{self, InferOpts, LlmClient};

// ─── Skill names ──────────────────────────────────────────────────────────────

const SKILL_1A: &str = "para-projects-areas";
const SKILL_1B: &str = "para-resource-entities";
const SKILL_2: &str = "sb-atomize";

// ─── Public entry point ───────────────────────────────────────────────────────

pub async fn run_inbox_watcher(config: Arc<Config>, pool: DbPool) {
    let inbox = &config.inbox;

    for dir in [
        inbox.watch_dir.as_str(),
        inbox.archive_dir.as_str(),
        inbox.queue_dir.as_str(),
    ] {
        if let Err(e) = tokio::fs::create_dir_all(dir).await {
            eprintln!("[inbox] could not create dir '{dir}': {e}");
        }
    }

    eprintln!(
        "[inbox] watcher started — watching '{}' every {}s (skills: {}, queue: {})",
        inbox.watch_dir, inbox.poll_interval_secs, inbox.skills_dir, inbox.queue_dir
    );

    let interval = Duration::from_secs(inbox.poll_interval_secs);
    loop {
        if let Err(e) = scan_and_process(&config, &pool).await {
            eprintln!("[inbox] scan error: {e}");
        }
        sleep(interval).await;
    }
}

// ─── Scan inbox directory ────────────────────────────────────────────────────

async fn scan_and_process(config: &Arc<Config>, pool: &DbPool) -> Result<()> {
    let watch_dir = Path::new(&config.inbox.watch_dir);
    let mut entries = fs::read_dir(watch_dir)
        .await
        .with_context(|| format!("reading inbox dir '{}'", watch_dir.display()))?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if path.is_file() && (ext == "md" || ext == "txt") {
            if let Err(e) = process_file(config, pool, &path).await {
                eprintln!("[inbox] failed to process '{}': {e}", path.display());
            }
        }
    }
    Ok(())
}

// ─── Process one file ────────────────────────────────────────────────────────

async fn process_file(config: &Arc<Config>, _pool: &DbPool, source_path: &Path) -> Result<()> {
    let source_content = fs::read_to_string(source_path)
        .await
        .with_context(|| format!("reading '{}'", source_path.display()))?;

    let filename = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled");
    let slug = to_slug(filename);
    let source_id = compute_source_id(source_content.trim());
    let generated_at = chrono::Utc::now().to_rfc3339();
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();

    let archive_dir = PathBuf::from(&config.inbox.archive_dir).join(format!("{slug}-{timestamp}"));
    fs::create_dir_all(&archive_dir)
        .await
        .with_context(|| format!("creating archive dir '{}'", archive_dir.display()))?;

    let log_path = archive_dir.join("pipeline.log");

    log_event(
        &log_path,
        serde_json::json!({
            "event": "pipeline_start", "slug": slug,
            "archive": archive_dir.display().to_string(),
            "source_file": source_path.display().to_string(), "ts": &generated_at,
        }),
    )
    .await;

    // Stage 0: move source into archive
    let archived_source = archive_dir.join("source.md");
    fs::rename(source_path, &archived_source)
        .await
        .with_context(|| format!("moving source to '{}'", archived_source.display()))?;

    // Build LLM client
    let backend = config
        .inbox
        .llm_backend
        .as_deref()
        .unwrap_or(&config.llm.backend);
    let llm: Box<dyn LlmClient> = match llm::build_client_for_backend(backend, &config.llm) {
        Ok(c) => c,
        Err(e) => {
            fail_pipeline(&log_path, "llm_init", &e.to_string()).await;
            return Err(e);
        }
    };

    let skills_dir = Path::new(&config.inbox.skills_dir);

    // Checkpoint paths
    let pa_toc_path = archive_dir.join("projects-areas-toc.md");
    let pa_typed_path = archive_dir.join("projects-areas-typed.md");
    let res_toc_path = archive_dir.join("resources-toc.md");
    let res_typed_path = archive_dir.join("resources-typed.md");
    let atomized_path = archive_dir.join(format!("{slug}-atomized.md"));

    let checkpoints_exist = pa_toc_path.exists()
        && pa_typed_path.exists()
        && res_toc_path.exists()
        && res_typed_path.exists();

    if checkpoints_exist && atomized_path.exists() {
        eprintln!("[inbox] '{}' already fully processed — skipping", slug);
        return Ok(());
    }

    // ── Stage 1: para-projects-areas + para-resource-entities (parallel) ──────
    if !checkpoints_exist {
        eprintln!(
            "[inbox] [{slug}] Stage 1 — para-process (reading skills from {})",
            skills_dir.display()
        );

        let prompt_1a = match build_stage1a_prompt(
            skills_dir,
            &source_content,
            &source_id,
            &generated_at,
        )
        .await
        {
            Ok(p) => p,
            Err(e) => {
                fail_pipeline(&log_path, "skill_load_1a", &e.to_string()).await;
                return Err(e);
            }
        };
        let prompt_1b = match build_stage1b_prompt(
            skills_dir,
            &source_content,
            &source_id,
            &generated_at,
        )
        .await
        {
            Ok(p) => p,
            Err(e) => {
                fail_pipeline(&log_path, "skill_load_1b", &e.to_string()).await;
                return Err(e);
            }
        };

        log_event(&log_path, serde_json::json!({"event":"stage_start","stage":"para-projects-areas","ts":chrono::Utc::now().to_rfc3339()})).await;
        log_event(&log_path, serde_json::json!({"event":"stage_start","stage":"para-resource-entities","ts":chrono::Utc::now().to_rfc3339()})).await;

        let opts = InferOpts {
            temperature: 0.1,
            max_tokens: 8192,
            json_mode: false,
        };
        let (r1a, r1b) = tokio::join!(
            llm.infer(&prompt_1a, opts.clone()),
            llm.infer(&prompt_1b, opts),
        );

        // Handle 1a
        match r1a.and_then(|o| parse_two_file_output(&o)) {
            Ok((toc, typed)) => {
                fs::write(&pa_toc_path, &toc).await?;
                fs::write(&pa_typed_path, &typed).await?;
                log_event(&log_path, serde_json::json!({"event":"stage_end","stage":"para-projects-areas","ok":true,"ts":chrono::Utc::now().to_rfc3339()})).await;
            }
            Err(e) => {
                log_event(&log_path, serde_json::json!({"event":"stage_end","stage":"para-projects-areas","ok":false,"error":e.to_string(),"ts":chrono::Utc::now().to_rfc3339()})).await;
                fail_pipeline(&log_path, "para-projects-areas", &e.to_string()).await;
                return Err(e);
            }
        }

        // Handle 1b
        match r1b.and_then(|o| parse_two_file_output(&o)) {
            Ok((typed, toc)) => {
                fs::write(&res_typed_path, &typed).await?;
                fs::write(&res_toc_path, &toc).await?;
                log_event(&log_path, serde_json::json!({"event":"stage_end","stage":"para-resource-entities","ok":true,"ts":chrono::Utc::now().to_rfc3339()})).await;
            }
            Err(e) => {
                log_event(&log_path, serde_json::json!({"event":"stage_end","stage":"para-resource-entities","ok":false,"error":e.to_string(),"ts":chrono::Utc::now().to_rfc3339()})).await;
                fail_pipeline(&log_path, "para-resource-entities", &e.to_string()).await;
                return Err(e);
            }
        }
    } else {
        eprintln!("[inbox] [{slug}] Stage 1 checkpoints found — skipping to Stage 2");
    }

    // ── Stage 2: sb-atomize ──────────────────────────────────────────────────
    eprintln!("[inbox] [{slug}] Stage 2 — sb-atomize");

    let pa_toc = fs::read_to_string(&pa_toc_path).await?;
    let pa_typed = fs::read_to_string(&pa_typed_path).await?;
    let res_toc = fs::read_to_string(&res_toc_path).await?;
    let res_typed = fs::read_to_string(&res_typed_path).await?;
    let source_title = slug_to_title(&slug);

    let prompt_2 = match build_stage2_prompt(
        skills_dir,
        &source_content,
        &source_id,
        &generated_at,
        &source_title,
        &slug,
        &pa_toc,
        &pa_typed,
        &res_toc,
        &res_typed,
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            fail_pipeline(&log_path, "skill_load_2", &e.to_string()).await;
            return Err(e);
        }
    };

    log_event(&log_path, serde_json::json!({"event":"stage_start","stage":"sb-atomize","ts":chrono::Utc::now().to_rfc3339()})).await;

    let opts2 = InferOpts {
        temperature: 0.15,
        max_tokens: 16384,
        json_mode: false,
    };
    match llm
        .infer(&prompt_2, opts2)
        .await
        .and_then(|o| parse_two_file_output(&o))
    {
        Ok((atomized, toc_manifest)) => {
            let toc_path = archive_dir.join(format!("{slug}-toc.md"));
            fs::write(&atomized_path, &atomized).await?;
            fs::write(&toc_path, &toc_manifest).await?;
            log_event(&log_path, serde_json::json!({"event":"stage_end","stage":"sb-atomize","ok":true,"ts":chrono::Utc::now().to_rfc3339()})).await;

            // ── Stage 3: enqueue ─────────────────────────────────────────────
            // Write the atomized file to q/ — the queue watcher ingests it.
            eprintln!("[inbox] [{slug}] Stage 3 — enqueue");
            log_event(&log_path, serde_json::json!({"event":"stage_start","stage":"enqueue","ts":chrono::Utc::now().to_rfc3339()})).await;

            let queue_path =
                PathBuf::from(&config.inbox.queue_dir).join(format!("{slug}-atomized.md"));

            match fs::write(&queue_path, &atomized).await {
                Ok(()) => {
                    log_event(
                        &log_path,
                        serde_json::json!({
                            "event": "pipeline_end", "status": "queued",
                            "queue_file": queue_path.display().to_string(),
                            "ts": chrono::Utc::now().to_rfc3339()
                        }),
                    )
                    .await;
                    eprintln!("[inbox] [{slug}] ✓ queued → {}", queue_path.display());
                }
                Err(e) => {
                    fail_pipeline(&log_path, "enqueue", &e.to_string()).await;
                    eprintln!("[inbox] [{slug}] enqueue failed: {e}");
                    eprintln!(
                        "[inbox]   atomized file preserved: {}",
                        atomized_path.display()
                    );
                }
            }
        }
        Err(e) => {
            log_event(&log_path, serde_json::json!({"event":"stage_end","stage":"sb-atomize","ok":false,"error":e.to_string(),"ts":chrono::Utc::now().to_rfc3339()})).await;
            fail_pipeline(&log_path, "sb-atomize", &e.to_string()).await;
            eprintln!("[inbox] [{slug}] Stage 2 failed: {e}");
            eprintln!(
                "[inbox]   Stage 1 checkpoints preserved in: {}",
                archive_dir.display()
            );
            return Err(e);
        }
    }

    Ok(())
}

// ─── Skill loading ────────────────────────────────────────────────────────────

/// Recursively collect all .md files under `dir`, sorted for determinism.
async fn collect_md_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    if !dir.exists() {
        return Ok(results);
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let mut entries = fs::read_dir(&current)
            .await
            .with_context(|| format!("reading dir '{}'", current.display()))?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                results.push(path);
            }
        }
    }
    results.sort();
    Ok(results)
}

/// Load a skill by reading its SKILL.md then all files in its references/ tree.
/// Returns the concatenated content ready to embed in a prompt.
async fn load_skill(skills_dir: &Path, skill_subpath: &str) -> Result<String> {
    let skill_dir = skills_dir.join(skill_subpath);
    let skill_md_path = skill_dir.join("SKILL.md");

    let skill_md = fs::read_to_string(&skill_md_path).await.with_context(|| {
        format!(
            "reading skill file '{}'. \
             Ensure skills_dir is set correctly in anansi.toml \
             (default /data/skills = mount point for r2-anansi.plugin/)",
            skill_md_path.display()
        )
    })?;

    let mut assembled = skill_md;

    let refs_dir = skill_dir.join("references");
    let ref_files = collect_md_files(&refs_dir).await?;

    for path in ref_files {
        let rel = path.strip_prefix(&skill_dir).unwrap_or(&path);
        let content = fs::read_to_string(&path)
            .await
            .with_context(|| format!("reading reference file '{}'", path.display()))?;
        assembled.push_str(&format!(
            "\n\n---\n## [Reference: {}]\n\n{}",
            rel.display(),
            content
        ));
    }

    Ok(assembled)
}

// ─── Prompt builders ─────────────────────────────────────────────────────────

/// System preamble prepended to every skill prompt when running server-side.
/// Instructs the LLM to use <<<FILE:>>> delimiters instead of creating real files.
const PIPELINE_PREAMBLE: &str = r#"[PIPELINE CONTEXT]
You are the anansi knowledge pipeline running this skill server-side in a fully automated, non-interactive context. There is no human in the loop for this run.

**Output format override (required for machine parsing):**
Instead of creating files, output your two files separated by <<<FILE:filename>>> markers — no text before the first marker, no text after the last file's content. Example:

<<<FILE:first-output-filename.md>>>
[content of first file]
<<<FILE:second-output-filename.md>>>
[content of second file]

The source document, runtime metadata, and skill instructions follow.

---
"#;

async fn build_stage1a_prompt(
    skills_dir: &Path,
    source: &str,
    source_id: &str,
    generated_at: &str,
) -> Result<String> {
    let skill = load_skill(skills_dir, SKILL_1A).await?;
    Ok(format!(
        "{PIPELINE_PREAMBLE}\
         [RUNTIME METADATA]\n\
         source_id: {source_id}\n\
         generated_at: {generated_at}\n\n\
         [SKILL INSTRUCTIONS AND REFERENCES]\n\
         {skill}\n\n\
         ---\n\
         [SOURCE DOCUMENT]\n\n\
         {source}"
    ))
}

async fn build_stage1b_prompt(
    skills_dir: &Path,
    source: &str,
    source_id: &str,
    generated_at: &str,
) -> Result<String> {
    let skill = load_skill(skills_dir, SKILL_1B).await?;
    Ok(format!(
        "{PIPELINE_PREAMBLE}\
         [RUNTIME METADATA]\n\
         source_id: {source_id}\n\
         generated_at: {generated_at}\n\n\
         [SKILL INSTRUCTIONS AND REFERENCES]\n\
         {skill}\n\n\
         ---\n\
         [SOURCE DOCUMENT]\n\n\
         {source}"
    ))
}

#[allow(clippy::too_many_arguments)]
async fn build_stage2_prompt(
    skills_dir: &Path,
    source: &str,
    source_id: &str,
    generated_at: &str,
    source_title: &str,
    source_slug: &str,
    pa_toc: &str,
    pa_typed: &str,
    res_toc: &str,
    res_typed: &str,
) -> Result<String> {
    let skill = load_skill(skills_dir, SKILL_2).await?;
    Ok(format!(
        "{PIPELINE_PREAMBLE}\
         [RUNTIME METADATA]\n\
         source_id: {source_id}\n\
         generated_at: {generated_at}\n\
         source_title: {source_title}\n\
         source_slug: {source_slug}\n\n\
         [STAGE 1 OUTPUTS]\n\n\
         ### projects-areas-toc.md\n{pa_toc}\n\n\
         ### projects-areas-typed.md\n{pa_typed}\n\n\
         ### resources-toc.md\n{res_toc}\n\n\
         ### resources-typed.md\n{res_typed}\n\n\
         [SKILL INSTRUCTIONS AND REFERENCES]\n\
         {skill}\n\n\
         ---\n\
         [SOURCE DOCUMENT]\n\n\
         {source}"
    ))
}

// ─── Output parser ───────────────────────────────────────────────────────────

const FILE_MARKER: &str = "<<<FILE:";

fn parse_two_file_output(output: &str) -> Result<(String, String)> {
    let mut markers: Vec<usize> = Vec::new();
    let mut pos = 0;
    while let Some(idx) = output[pos..].find(FILE_MARKER) {
        markers.push(pos + idx);
        pos += idx + FILE_MARKER.len();
    }

    if markers.len() < 2 {
        return Err(anyhow::anyhow!(
            "LLM output did not contain two <<<FILE:>>> markers (found {}). First 400 chars: {}",
            markers.len(),
            &output[..output.len().min(400)]
        ));
    }

    let first_start = output[markers[0]..]
        .find('\n')
        .map(|n| markers[0] + n + 1)
        .unwrap_or(markers[0]);
    let first_content = output[first_start..markers[1]].trim().to_string();

    let second_start = output[markers[1]..]
        .find('\n')
        .map(|n| markers[1] + n + 1)
        .unwrap_or(markers[1]);
    let second_content = output[second_start..].trim().to_string();

    if first_content.is_empty() || second_content.is_empty() {
        return Err(anyhow::anyhow!(
            "One or both file blocks are empty in LLM output"
        ));
    }

    Ok((first_content, second_content))
}

// ─── Slug utilities ──────────────────────────────────────────────────────────

pub fn to_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(60)
        .collect()
}

fn slug_to_title(slug: &str) -> String {
    slug.split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn compute_source_id(content: &str) -> String {
    let hash = Sha256::digest(content.as_bytes());
    format!("{:x}", hash)[..16].to_string()
}

// ─── Logging ─────────────────────────────────────────────────────────────────

async fn log_event(log_path: &Path, event: serde_json::Value) {
    use tokio::io::AsyncWriteExt;
    let line = format!("{}\n", event);
    if let Ok(mut f) = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .await
    {
        let _ = f.write_all(line.as_bytes()).await;
    }
}

async fn fail_pipeline(log_path: &Path, stage: &str, error: &str) {
    log_event(
        log_path,
        serde_json::json!({
            "event": "pipeline_end", "status": "failed",
            "failed_stage": stage, "error": error,
            "ts": chrono::Utc::now().to_rfc3339(),
        }),
    )
    .await;
}
