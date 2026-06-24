//! anansi-crawl background watcher (Build-13).
//!
//! Periodically runs the mechanical wiki maintenance pass (`WikiStore::crawl`):
//! reconcile the wiki projection against canonical Postgres state, garbage-
//! collect orphan files, rebuild `index.md`, and journal a summary. Mirrors the
//! `queue.rs` / `inbox.rs` watcher pattern. Spawned only when both
//! `wiki.enabled` and `wiki.crawl_enabled` are set.

use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;

use crate::config::Config;
use crate::db::DbPool;
use crate::wiki::WikiStore;

pub async fn run_crawl_watcher(config: Arc<Config>, pool: DbPool) {
    let wiki = WikiStore::from_config(&config);
    let interval = Duration::from_secs(config.wiki.crawl_interval_secs.max(1));

    eprintln!(
        "[crawl] watcher started — every {}s",
        config.wiki.crawl_interval_secs
    );

    loop {
        // Run on startup first (self-heals the wiki), then on each interval.
        match wiki.crawl(&pool).await {
            Ok(report) => eprintln!(
                "[crawl] done — {} notes projected, {} orphans removed, {} errors",
                report.notes_projected, report.orphans_removed, report.errors
            ),
            Err(e) => eprintln!("[crawl] failed: {e}"),
        }
        sleep(interval).await;
    }
}
