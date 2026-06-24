/// Queue watcher — polls `/data/q/` and ingests every atomized `.md` file it finds.
///
/// Both the inbox watcher pipeline and the `anansi_ingest_atomized` MCP tool
/// write atomized files to this directory.  The queue watcher is the single
/// consumer: it calls `ingest_atomized()` for each file, then moves it to
/// `q/processed/` on success or `q/failed/` on error.
///
/// Directory layout:
///   /data/q/
///     <slug>-atomized.md      ← dropped by inbox watcher
///     <uuid>.md               ← dropped by anansi_ingest_atomized MCP tool
///     processed/              ← successfully ingested files
///     failed/                 ← files that failed ingestion (error appended)
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::fs;
use tokio::time::sleep;

use crate::atomized_ingest::ingest_atomized;
use crate::config::Config;
use crate::db::DbPool;
use crate::wiki::WikiStore;

// ─── Public entry point ───────────────────────────────────────────────────────

pub async fn run_queue_watcher(config: Arc<Config>, pool: DbPool) {
    let queue_dir = &config.inbox.queue_dir;
    let processed_dir = format!("{queue_dir}/processed");
    let failed_dir = format!("{queue_dir}/failed");

    for dir in [
        queue_dir.as_str(),
        processed_dir.as_str(),
        failed_dir.as_str(),
    ] {
        if let Err(e) = fs::create_dir_all(dir).await {
            eprintln!("[queue] could not create dir '{dir}': {e}");
        }
    }

    eprintln!(
        "[queue] watcher started — polling '{queue_dir}' every {}s",
        config.inbox.queue_poll_interval_secs
    );

    // Built once from config; covers the queue-direct path AND the inbox
    // pipeline (inbox enqueues here; this watcher is the sole ingest consumer).
    let wiki = WikiStore::from_config(&config);

    let interval = Duration::from_secs(config.inbox.queue_poll_interval_secs);
    loop {
        if let Err(e) = scan_and_ingest(&config, &pool, &wiki).await {
            eprintln!("[queue] scan error: {e}");
        }
        sleep(interval).await;
    }
}

// ─── Scan queue directory ────────────────────────────────────────────────────

async fn scan_and_ingest(config: &Arc<Config>, pool: &DbPool, wiki: &WikiStore) -> Result<()> {
    let queue_dir = Path::new(&config.inbox.queue_dir);
    let processed_dir = queue_dir.join("processed");
    let failed_dir = queue_dir.join("failed");

    let mut entries = fs::read_dir(queue_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        // Only process .md files directly in queue_dir (skip sub-directories)
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" {
            continue;
        }

        if let Err(e) = process_queued_file(pool, &path, &processed_dir, &failed_dir, wiki).await {
            eprintln!("[queue] error processing '{}': {e}", path.display());
        }
    }

    Ok(())
}

// ─── Process one queued file ─────────────────────────────────────────────────

async fn process_queued_file(
    pool: &DbPool,
    path: &Path,
    processed_dir: &Path,
    failed_dir: &Path,
    wiki: &WikiStore,
) -> Result<()> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.md");

    let content = fs::read_to_string(path).await?;

    eprintln!("[queue] ingesting '{filename}'");

    match ingest_atomized(pool, &content, None, Some(&path.to_string_lossy()), Some(wiki)).await {
        Ok(report) if report.status == "already_ingested" => {
            eprintln!(
                "[queue] '{filename}' already ingested (source_id={}) — moving to processed/",
                report.source_id
            );
            move_file(path, &processed_dir.join(filename)).await;
        }
        Ok(report) => {
            eprintln!(
                "[queue] '{filename}' ✓ {} notes created (source_id={})",
                report.notes_created, report.source_id
            );
            move_file(path, &processed_dir.join(filename)).await;
        }
        Err(e) => {
            eprintln!("[queue] '{filename}' ✗ ingest failed: {e}");
            // Append error to the file and move to failed/
            let error_note = format!("\n\n<!-- queue-error: {} -->\n", e);
            let mut content_with_error = content;
            content_with_error.push_str(&error_note);
            let dest = failed_dir.join(filename);
            if let Err(we) = fs::write(&dest, content_with_error).await {
                eprintln!("[queue] could not write to failed/: {we}");
            } else {
                let _ = fs::remove_file(path).await;
            }
        }
    }

    Ok(())
}

async fn move_file(src: &Path, dest: &Path) {
    if let Err(e) = fs::rename(src, dest).await {
        eprintln!(
            "[queue] could not move '{}' → '{}': {e}",
            src.display(),
            dest.display()
        );
    }
}
