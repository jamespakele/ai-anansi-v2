/// Inbox watcher — background Tokio task that polls a watch directory and runs
/// the full para-process → sb-atomize → ingest_atomized pipeline on each file.
///
/// Directory layout per run:
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
use sha2::{Sha256, Digest};
use tokio::fs;
use tokio::time::sleep;

use crate::atomized_ingest::ingest_atomized;
use crate::config::Config;
use crate::db::DbPool;
use crate::llm::{self, InferOpts, LlmClient};

// ─── Prompt templates (embedded at compile time) ──────────────────────────────

const PROMPT_STAGE1A: &str = include_str!("../prompts/stage1a-projects-areas.txt");
const PROMPT_STAGE1B: &str = include_str!("../prompts/stage1b-resource-entities.txt");
const PROMPT_STAGE2:  &str = include_str!("../prompts/stage2-sb-atomize.txt");

// ─── File delimiter used in LLM output ───────────────────────────────────────

const FILE_MARKER: &str = "<<<FILE:";

// ─── Public entry point ───────────────────────────────────────────────────────

/// Spawns a polling loop that watches `config.inbox.watch_dir` and runs the
/// full pipeline on each new `.md` or `.txt` file it finds.
pub async fn run_inbox_watcher(config: Arc<Config>, pool: DbPool) {
    let inbox = &config.inbox;

    // Ensure watch and archive dirs exist
    if let Err(e) = tokio::fs::create_dir_all(&inbox.watch_dir).await {
        eprintln!("[inbox] could not create watch_dir '{}': {e}", inbox.watch_dir);
    }
    if let Err(e) = tokio::fs::create_dir_all(&inbox.archive_dir).await {
        eprintln!("[inbox] could not create archive_dir '{}': {e}", inbox.archive_dir);
    }

    eprintln!("[inbox] watcher started — watching '{}' every {}s",
        inbox.watch_dir, inbox.poll_interval_secs);

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
    let mut entries = fs::read_dir(watch_dir).await
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

async fn process_file(config: &Arc<Config>, pool: &DbPool, source_path: &Path) -> Result<()> {
    let source_content = fs::read_to_string(source_path).await
        .with_context(|| format!("reading '{}'", source_path.display()))?;

    // Compute slug from filename
    let filename = source_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled");
    let slug = to_slug(filename);

    // Compute metadata
    let source_id = compute_source_id(source_content.trim());
    let generated_at = chrono::Utc::now().to_rfc3339();
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let archive_name = format!("{slug}-{timestamp}");

    // Build archive dir
    let archive_dir = PathBuf::from(&config.inbox.archive_dir).join(&archive_name);
    fs::create_dir_all(&archive_dir).await
        .with_context(|| format!("creating archive dir '{}'", archive_dir.display()))?;

    let log_path = archive_dir.join("pipeline.log");

    log_event(&log_path, serde_json::json!({
        "event": "pipeline_start",
        "slug": slug,
        "archive": archive_dir.display().to_string(),
        "source_file": source_path.display().to_string(),
        "ts": &generated_at,
    })).await;

    // Stage 0: move source into archive
    let archived_source = archive_dir.join("source.md");
    fs::rename(source_path, &archived_source).await
        .with_context(|| format!("moving source to '{}'", archived_source.display()))?;

    // Build LLM client for inbox pipeline
    let backend = config.inbox.llm_backend.as_deref()
        .unwrap_or(&config.llm.backend);
    let llm: Box<dyn LlmClient> = match llm::build_client_for_backend(backend, &config.llm) {
        Ok(c) => c,
        Err(e) => {
            log_event(&log_path, serde_json::json!({
                "event": "pipeline_end", "status": "failed",
                "failed_stage": "llm_init", "error": e.to_string(),
                "ts": chrono::Utc::now().to_rfc3339(),
            })).await;
            return Err(e);
        }
    };

    // Check for checkpoint resume: do Stage 1 checkpoint files already exist?
    let pa_toc_path    = archive_dir.join("projects-areas-toc.md");
    let pa_typed_path  = archive_dir.join("projects-areas-typed.md");
    let res_toc_path   = archive_dir.join("resources-toc.md");
    let res_typed_path = archive_dir.join("resources-typed.md");
    let atomized_path  = archive_dir.join(format!("{slug}-atomized.md"));

    let checkpoints_exist = pa_toc_path.exists() && pa_typed_path.exists()
        && res_toc_path.exists() && res_typed_path.exists();

    if checkpoints_exist && atomized_path.exists() {
        eprintln!("[inbox] '{}' already fully processed — skipping", slug);
        return Ok(());
    }

    // ── Stage 1: para-projects-areas + para-resource-entities in parallel ────
    if !checkpoints_exist {
        eprintln!("[inbox] [{slug}] Stage 1 — running para-process");

        let infer_opts_1a = InferOpts { temperature: 0.1, max_tokens: 8192, json_mode: false };
        let infer_opts_1b = InferOpts { temperature: 0.1, max_tokens: 8192, json_mode: false };

        let prompt_1a = build_stage1a_prompt(&source_content, &source_id, &generated_at);
        let prompt_1b = build_stage1b_prompt(&source_content, &source_id, &generated_at);

        log_event(&log_path, serde_json::json!({
            "event": "stage_start", "stage": "para-projects-areas", "ts": chrono::Utc::now().to_rfc3339()
        })).await;
        log_event(&log_path, serde_json::json!({
            "event": "stage_start", "stage": "para-resource-entities", "ts": chrono::Utc::now().to_rfc3339()
        })).await;

        // Run both Stage 1 passes in parallel
        let (result_1a, result_1b) = tokio::join!(
            llm.infer(&prompt_1a, infer_opts_1a),
            llm.infer(&prompt_1b, infer_opts_1b),
        );

        // Handle Stage 1a output
        match result_1a {
            Ok(output) => {
                match parse_two_file_output(&output) {
                    Ok((toc, typed)) => {
                        fs::write(&pa_toc_path, &toc).await?;
                        fs::write(&pa_typed_path, &typed).await?;
                        log_event(&log_path, serde_json::json!({
                            "event": "stage_end", "stage": "para-projects-areas",
                            "ok": true, "ts": chrono::Utc::now().to_rfc3339()
                        })).await;
                    }
                    Err(e) => {
                        log_event(&log_path, serde_json::json!({
                            "event": "stage_end", "stage": "para-projects-areas",
                            "ok": false, "error": e.to_string(),
                            "ts": chrono::Utc::now().to_rfc3339()
                        })).await;
                        fail_pipeline(&log_path, "para-projects-areas", &e.to_string()).await;
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                log_event(&log_path, serde_json::json!({
                    "event": "stage_end", "stage": "para-projects-areas",
                    "ok": false, "error": e.to_string(),
                    "ts": chrono::Utc::now().to_rfc3339()
                })).await;
                fail_pipeline(&log_path, "para-projects-areas", &e.to_string()).await;
                return Err(e);
            }
        }

        // Handle Stage 1b output
        match result_1b {
            Ok(output) => {
                match parse_two_file_output(&output) {
                    Ok((toc, typed)) => {
                        fs::write(&res_toc_path, &toc).await?;
                        fs::write(&res_typed_path, &typed).await?;
                        log_event(&log_path, serde_json::json!({
                            "event": "stage_end", "stage": "para-resource-entities",
                            "ok": true, "ts": chrono::Utc::now().to_rfc3339()
                        })).await;
                    }
                    Err(e) => {
                        log_event(&log_path, serde_json::json!({
                            "event": "stage_end", "stage": "para-resource-entities",
                            "ok": false, "error": e.to_string(),
                            "ts": chrono::Utc::now().to_rfc3339()
                        })).await;
                        fail_pipeline(&log_path, "para-resource-entities", &e.to_string()).await;
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                log_event(&log_path, serde_json::json!({
                    "event": "stage_end", "stage": "para-resource-entities",
                    "ok": false, "error": e.to_string(),
                    "ts": chrono::Utc::now().to_rfc3339()
                })).await;
                fail_pipeline(&log_path, "para-resource-entities", &e.to_string()).await;
                return Err(e);
            }
        }
    } else {
        eprintln!("[inbox] [{slug}] Stage 1 checkpoint files found — skipping to Stage 2");
    }

    // ── Stage 2: sb-atomize ──────────────────────────────────────────────────
    eprintln!("[inbox] [{slug}] Stage 2 — sb-atomize");

    let pa_toc    = fs::read_to_string(&pa_toc_path).await?;
    let pa_typed  = fs::read_to_string(&pa_typed_path).await?;
    let res_toc   = fs::read_to_string(&res_toc_path).await?;
    let res_typed = fs::read_to_string(&res_typed_path).await?;

    // Derive source title from slug
    let source_title = slug_to_title(&slug);

    let prompt_2 = build_stage2_prompt(
        &source_content,
        &source_id,
        &generated_at,
        &source_title,
        &slug,
        &pa_toc,
        &pa_typed,
        &res_toc,
        &res_typed,
    );

    log_event(&log_path, serde_json::json!({
        "event": "stage_start", "stage": "sb-atomize", "ts": chrono::Utc::now().to_rfc3339()
    })).await;

    let infer_opts_2 = InferOpts { temperature: 0.15, max_tokens: 16384, json_mode: false };
    match llm.infer(&prompt_2, infer_opts_2).await {
        Ok(output) => {
            match parse_two_file_output(&output) {
                Ok((atomized, toc_manifest)) => {
                    let toc_path = archive_dir.join(format!("{slug}-toc.md"));
                    fs::write(&atomized_path, &atomized).await?;
                    fs::write(&toc_path, &toc_manifest).await?;
                    log_event(&log_path, serde_json::json!({
                        "event": "stage_end", "stage": "sb-atomize",
                        "ok": true, "ts": chrono::Utc::now().to_rfc3339()
                    })).await;

                    // ── Stage 3: ingest ──────────────────────────────────────
                    eprintln!("[inbox] [{slug}] Stage 3 — ingest");
                    log_event(&log_path, serde_json::json!({
                        "event": "stage_start", "stage": "ingest", "ts": chrono::Utc::now().to_rfc3339()
                    })).await;

                    match ingest_atomized(
                        pool,
                        &atomized,
                        Some(&toc_manifest),
                        Some(&archived_source.to_string_lossy()),
                    ).await {
                        Ok(result) => {
                            log_event(&log_path, serde_json::json!({
                                "event": "stage_end", "stage": "ingest",
                                "ok": true,
                                "source_id": result.source_id,
                                "notes_created": result.notes_created,
                                "ts": chrono::Utc::now().to_rfc3339(),
                            })).await;
                            log_event(&log_path, serde_json::json!({
                                "event": "pipeline_end", "status": "ok",
                                "source_id": result.source_id,
                                "notes_created": result.notes_created,
                                "archive": archive_dir.display().to_string(),
                                "ts": chrono::Utc::now().to_rfc3339(),
                            })).await;
                            eprintln!("[inbox] [{slug}] ✓ done — {} notes created (source_id={})",
                                result.notes_created, result.source_id);
                        }
                        Err(e) => {
                            log_event(&log_path, serde_json::json!({
                                "event": "stage_end", "stage": "ingest",
                                "ok": false, "error": e.to_string(),
                                "ts": chrono::Utc::now().to_rfc3339()
                            })).await;
                            fail_pipeline(&log_path, "ingest", &e.to_string()).await;
                            eprintln!("[inbox] [{slug}] ingest failed: {e}");
                            eprintln!("[inbox]   atomized file: {}", atomized_path.display());
                            eprintln!("[inbox]   retry via MCP: anansi_ingest_atomized");
                        }
                    }
                }
                Err(e) => {
                    log_event(&log_path, serde_json::json!({
                        "event": "stage_end", "stage": "sb-atomize",
                        "ok": false, "error": e.to_string(),
                        "ts": chrono::Utc::now().to_rfc3339()
                    })).await;
                    fail_pipeline(&log_path, "sb-atomize", &e.to_string()).await;
                    return Err(e);
                }
            }
        }
        Err(e) => {
            log_event(&log_path, serde_json::json!({
                "event": "stage_end", "stage": "sb-atomize",
                "ok": false, "error": e.to_string(),
                "ts": chrono::Utc::now().to_rfc3339()
            })).await;
            fail_pipeline(&log_path, "sb-atomize", &e.to_string()).await;
            eprintln!("[inbox] [{slug}] Stage 2 failed: {e}");
            eprintln!("[inbox]   Stage 1 checkpoint files preserved in: {}", archive_dir.display());
            eprintln!("[inbox]   Drop source back into inbox to resume from Stage 2");
            return Err(e);
        }
    }

    Ok(())
}

// ─── Prompt builders ─────────────────────────────────────────────────────────

fn build_stage1a_prompt(source: &str, source_id: &str, generated_at: &str) -> String {
    PROMPT_STAGE1A
        .replace("{{SOURCE}}", source)
        .replace("{{SOURCE_ID}}", source_id)
        .replace("{{GENERATED_AT}}", generated_at)
}

fn build_stage1b_prompt(source: &str, source_id: &str, generated_at: &str) -> String {
    PROMPT_STAGE1B
        .replace("{{SOURCE}}", source)
        .replace("{{SOURCE_ID}}", source_id)
        .replace("{{GENERATED_AT}}", generated_at)
}

#[allow(clippy::too_many_arguments)]
fn build_stage2_prompt(
    source: &str,
    source_id: &str,
    generated_at: &str,
    source_title: &str,
    source_slug: &str,
    pa_toc: &str,
    pa_typed: &str,
    res_toc: &str,
    res_typed: &str,
) -> String {
    PROMPT_STAGE2
        .replace("{{SOURCE}}", source)
        .replace("{{SOURCE_ID}}", source_id)
        .replace("{{GENERATED_AT}}", generated_at)
        .replace("{{SOURCE_TITLE}}", source_title)
        .replace("{{SOURCE_SLUG}}", source_slug)
        .replace("{{PROJECTS_AREAS_TOC}}", pa_toc)
        .replace("{{PROJECTS_AREAS_TYPED}}", pa_typed)
        .replace("{{RESOURCES_TOC}}", res_toc)
        .replace("{{RESOURCES_TYPED}}", res_typed)
}

// ─── Output parser ───────────────────────────────────────────────────────────

/// Parse LLM output that contains two <<<FILE:filename>>> delimited blocks.
/// Returns (first_file_content, second_file_content).
fn parse_two_file_output(output: &str) -> Result<(String, String)> {
    // Find all FILE: markers
    let mut markers: Vec<usize> = Vec::new();
    let mut pos = 0;
    while let Some(idx) = output[pos..].find(FILE_MARKER) {
        markers.push(pos + idx);
        pos += idx + FILE_MARKER.len();
    }

    if markers.len() < 2 {
        return Err(anyhow::anyhow!(
            "LLM output did not contain two <<<FILE:>>> markers. Found {}. \
             First 500 chars: {}",
            markers.len(),
            &output[..output.len().min(500)]
        ));
    }

    // Extract content between first marker (after its header line) and second marker
    let first_start = output[markers[0]..].find('\n')
        .map(|n| markers[0] + n + 1)
        .unwrap_or(markers[0]);
    let first_content = output[first_start..markers[1]].trim().to_string();

    // Extract content after second marker's header line to end of output
    let second_start = output[markers[1]..].find('\n')
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

/// Convert a filename stem to a kebab-case slug (anansi match_key algorithm).
pub fn to_slug(name: &str) -> String {
    let lower = name.to_lowercase();
    let replaced: String = lower.chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    replaced.split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(60)
        .collect()
}

/// Convert a slug back to a title-case display name (best-effort).
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

/// First 16 hex chars of SHA-256 of the trimmed source content.
fn compute_source_id(content: &str) -> String {
    let hash = Sha256::digest(content.as_bytes());
    format!("{:x}", hash)[..16].to_string()
}

// ─── Logging ─────────────────────────────────────────────────────────────────

async fn log_event(log_path: &Path, event: serde_json::Value) {
    use tokio::io::AsyncWriteExt;
    let line = format!("{}\n", event);
    if let Ok(mut f) = tokio::fs::OpenOptions::new()
        .create(true).append(true).open(log_path).await
    {
        let _ = f.write_all(line.as_bytes()).await;
    }
}

async fn fail_pipeline(log_path: &Path, stage: &str, error: &str) {
    log_event(log_path, serde_json::json!({
        "event": "pipeline_end",
        "status": "failed",
        "failed_stage": stage,
        "error": error,
        "ts": chrono::Utc::now().to_rfc3339(),
    })).await;
}

