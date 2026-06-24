use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use anansi2::config::Config;
use anansi2::crawl;
use anansi2::db;
use anansi2::inbox;
use anansi2::queue;
use anansi2::llm;
use anansi2::mcp;
use anansi2::pipeline::{ingest, IngestContext};
use anansi2::rules::RuleRegistry;
use anansi2::template::TemplateRegistry;
use anansi2::vault::Vault;
use anansi2::wiki::WikiStore;

// ---------------------------------------------------------------------------
// Embedded seed files (compile-time include_str!)
// ---------------------------------------------------------------------------

// Templates (37 files) — canonical source: llm/plugins/anansi.plugin/references/templates/
// utility
const TMPL_ACTION_ITEM_LIST: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/action_item_list.md");
const TMPL_ANANSI_CONFIG: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/anansi-config.md");
const TMPL_CONTAINER: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/container.md");
const TMPL_CONTEXT: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/context.md");
const TMPL_EVENT: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/event.md");
const TMPL_MEMO: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/memo.md");
const TMPL_OUTLINE: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/outline.md");
const TMPL_SOCIAL_POST: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/social-post.md");
const TMPL_SPEECH: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/speech.md");
const TMPL_TASK: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/task.md");
// identity (entity-*)
const TMPL_AREA: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/entity-area.md");
const TMPL_BOOK: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/entity-book.md");
const TMPL_FLIGHT: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/entity-flight.md");
const TMPL_FLIGHT_OUTPUT: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/entity-flight-output.md");
const TMPL_MISSION: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/entity-mission.md");
const TMPL_NOTE: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/entity-note.md");
const TMPL_OPERATION: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/entity-operation.md");
const TMPL_ORGANIZATION: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/entity-organization.md");
const TMPL_PERSON: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/entity-person.md");
const TMPL_PROJECT: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/entity-project.md");
const TMPL_TOPIC: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/identity-topic.md");
// source family
const TMPL_COMPANY_UPDATE: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/company-update.md");
const TMPL_EMAIL_THREAD: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/email-thread.md");
const TMPL_MEETING_SUMMARY: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/meeting-summary.md");
const TMPL_NEWSLETTER: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/newsletter.md");
const TMPL_PRESENTATION: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/presentation.md");
const TMPL_RESEARCH_PAPER: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/research-paper.md");
const TMPL_YOUTUBE_VIDEO: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/youtube-video.md");
// content_unit family
const TMPL_BOOK_CHAPTER: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/book-chapter.md");
const TMPL_BOOK_SECTION: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/book-section.md");
const TMPL_COMPANY_UPDATE_ITEM: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/company-update-item.md");
const TMPL_EMAIL_EXCHANGE: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/email-exchange.md");
const TMPL_MEETING_TOPIC_DISCUSSION: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/meeting-topic-discussion.md");
const TMPL_NEWSLETTER_ITEM: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/newsletter-item.md");
const TMPL_PRESENTATION_SLIDE: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/presentation-slide.md");
const TMPL_RESEARCH_SECTION: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/research-section.md");
const TMPL_YOUTUBE_CHAPTER: &str = include_str!("../llm/plugins/anansi.plugin/references/templates/youtube-chapter.md");

// Rules (4 files)
const RULE_ATOMICITY: &str = include_str!("../%Rules/%Atomicity.md");
const RULE_DOWNSTREAM_FLOW: &str = include_str!("../%Rules/%Downstream-Flow.md");
const RULE_MERGE_STRATEGY: &str = include_str!("../%Rules/%Merge-Strategy.md");
const RULE_TEMPLATE_SCHEMA: &str = include_str!("../%Rules/%Template-Schema.md");

// Example config
const ANANSI_TOML_EXAMPLE: &str = include_str!("../anansi.toml.example");

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "anansi2", about = "Anansi v2 knowledge pipeline CLI")]
struct Cli {
    /// Root directory of the anansi vault (defaults to current directory)
    #[arg(long, global = true, default_value = ".")]
    root: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialise a new anansi vault at --root
    Init,
    /// Ingest a source file into the vault
    Ingest {
        /// Path to the source markdown file
        file: PathBuf,
    },
    /// Start the MCP HTTP server
    Serve,
    /// Manage the local LLM-wiki (projection of Postgres at the configured
    /// [wiki] dir). For a LOCAL anansi install; remote installs use the
    /// anansi-init-wiki skill instead.
    Wiki {
        #[command(subcommand)]
        action: WikiAction,
    },
}

#[derive(Subcommand)]
enum WikiAction {
    /// Create the wiki dir and populate it from Postgres. Idempotent — also
    /// refreshes an existing wiki (repairs/updates files, GCs stale ones).
    Init,
    /// Wipe existing wiki files (*.md + .lint-state) then regenerate from
    /// Postgres — for corruption or a guaranteed-fresh tree.
    Rebuild,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init => cmd_init(&cli.root).await,
        Command::Ingest { file } => cmd_ingest(&cli.root, &file).await,
        Command::Serve => cmd_serve(&cli.root).await,
        Command::Wiki { action } => match action {
            WikiAction::Init => cmd_rebuild_wiki(&cli.root, false).await,
            WikiAction::Rebuild => cmd_rebuild_wiki(&cli.root, true).await,
        },
    }
}

// ---------------------------------------------------------------------------
// Shared seed template table — (filename, entity_type, content)
// Used by both cmd_init (file seeding) and seed_templates_to_db (DB seeding).
// ---------------------------------------------------------------------------

const SEED_TEMPLATES: &[(&str, &str)] = &[
    // utility
    ("action_item_list.md", TMPL_ACTION_ITEM_LIST),
    ("anansi-config.md", TMPL_ANANSI_CONFIG),
    ("container.md", TMPL_CONTAINER),
    ("context.md", TMPL_CONTEXT),
    ("event.md", TMPL_EVENT),
    ("memo.md", TMPL_MEMO),
    ("outline.md", TMPL_OUTLINE),
    ("social-post.md", TMPL_SOCIAL_POST),
    ("speech.md", TMPL_SPEECH),
    ("task.md", TMPL_TASK),
    // identity
    ("entity-area.md", TMPL_AREA),
    ("entity-book.md", TMPL_BOOK),
    ("entity-flight.md", TMPL_FLIGHT),
    ("entity-flight-output.md", TMPL_FLIGHT_OUTPUT),
    ("entity-mission.md", TMPL_MISSION),
    ("entity-note.md", TMPL_NOTE),
    ("entity-operation.md", TMPL_OPERATION),
    ("entity-organization.md", TMPL_ORGANIZATION),
    ("entity-person.md", TMPL_PERSON),
    ("entity-project.md", TMPL_PROJECT),
    ("identity-topic.md", TMPL_TOPIC),
    // source
    ("company-update.md", TMPL_COMPANY_UPDATE),
    ("email-thread.md", TMPL_EMAIL_THREAD),
    ("meeting-summary.md", TMPL_MEETING_SUMMARY),
    ("newsletter.md", TMPL_NEWSLETTER),
    ("presentation.md", TMPL_PRESENTATION),
    ("research-paper.md", TMPL_RESEARCH_PAPER),
    ("youtube-video.md", TMPL_YOUTUBE_VIDEO),
    // content_unit
    ("book-chapter.md", TMPL_BOOK_CHAPTER),
    ("book-section.md", TMPL_BOOK_SECTION),
    ("company-update-item.md", TMPL_COMPANY_UPDATE_ITEM),
    ("email-exchange.md", TMPL_EMAIL_EXCHANGE),
    ("meeting-topic-discussion.md", TMPL_MEETING_TOPIC_DISCUSSION),
    ("newsletter-item.md", TMPL_NEWSLETTER_ITEM),
    ("presentation-slide.md", TMPL_PRESENTATION_SLIDE),
    ("research-section.md", TMPL_RESEARCH_SECTION),
    ("youtube-chapter.md", TMPL_YOUTUBE_CHAPTER),
];

/// Seed template definitions into the DB as anansi_config notes.
/// Policy: insert only if the match_key doesn't already exist — never overwrite.
async fn seed_templates_to_db(pool: &db::DbPool, templates: &[(&str, &str)]) -> Result<(u32, u32)> {
    let mut created = 0u32;
    let mut skipped = 0u32;

    for (_filename, content) in templates {
        // Parse the entity_type from the template's YAML frontmatter
        let entity_type = match anansi2::template::parse_template(content) {
            Ok(tmpl) => tmpl.entity_type.clone(),
            Err(e) => {
                eprintln!("WARN: skipping seed template {_filename}: {e}");
                skipped += 1;
                continue;
            }
        };

        let match_key = format!("anansi_config:template:{entity_type}");

        // Check if already present
        if db::find_note_by_match_key(pool, &match_key).await?.is_some() {
            skipped += 1;
            continue;
        }

        let now = db::now_rfc3339();
        let rec = db::NoteRecord {
            id: uuid::Uuid::new_v4().to_string(),
            entity_type: "anansi_config".to_string(),
            name: format!("Template: {entity_type}"),
            match_key,
            lede: Some(format!("Template definition for entity type '{entity_type}'")),
            why: None,
            content: Some(content.to_string()),
            has_conflicts: 0,
            conflicts_updated_at: None,
            merge_category: "pure_atomic".to_string(),
            created_from: db::MANUAL_SOURCE_ID.to_string(),
            source_count: 1,
            created_at: now.clone(),
            updated_at: now,
        };
        db::insert_note(pool, &rec).await?;
        created += 1;
    }

    Ok((created, skipped))
}

// ---------------------------------------------------------------------------
// cmd_init
// ---------------------------------------------------------------------------

async fn cmd_init(root: &Path) -> Result<()> {
    // Create directory tree under anansi/ subdirectory
    let dirs = ["anansi", "anansi/web", "anansi/%Rules", "anansi/templates"];
    for dir in &dirs {
        let p = root.join(dir);
        std::fs::create_dir_all(&p)
            .with_context(|| format!("creating directory {}", p.display()))?;
    }

    // Seed template files — NEVER overwrite an existing file.
    // Users may have customised templates; preserving them is intentional.
    let mut tmpl_created = 0u32;
    let mut tmpl_skipped = 0u32;
    for (name, content) in SEED_TEMPLATES {
        let dest = root.join("anansi").join("templates").join(name);
        if dest.exists() {
            tmpl_skipped += 1;
        } else {
            std::fs::write(&dest, content)
                .with_context(|| format!("writing template {}", dest.display()))?;
            tmpl_created += 1;
        }
    }

    // Seed rules files — same policy: skip if already present.
    let rules: &[(&str, &str)] = &[
        ("%Atomicity.md", RULE_ATOMICITY),
        ("%Downstream-Flow.md", RULE_DOWNSTREAM_FLOW),
        ("%Merge-Strategy.md", RULE_MERGE_STRATEGY),
        ("%Template-Schema.md", RULE_TEMPLATE_SCHEMA),
    ];
    let mut rules_created = 0u32;
    let mut rules_skipped = 0u32;
    for (name, content) in rules {
        let dest = root.join("anansi").join("%Rules").join(name);
        if dest.exists() {
            rules_skipped += 1;
        } else {
            std::fs::write(&dest, content)
                .with_context(|| format!("writing rule {}", dest.display()))?;
            rules_created += 1;
        }
    }

    // anansi.toml — skip if already present; new users get the example as a starting point.
    let toml_dest = root.join("anansi").join("anansi.toml");
    let toml_msg = if toml_dest.exists() {
        "preserved (your settings were not changed)"
    } else {
        std::fs::write(&toml_dest, ANANSI_TOML_EXAMPLE)
            .with_context(|| format!("writing {}", toml_dest.display()))?;
        "created — edit backend/model before first ingest"
    };

    // Open and migrate the database
    let config = Config::load(root)?;
    let db_pool = db::connect_and_migrate(&config.database_url).await
        .with_context(|| format!("connecting/migrating DB at {}", config.database_url))?;

    // Seed templates into the database as anansi_config notes
    let (db_created, db_skipped) = seed_templates_to_db(&db_pool, SEED_TEMPLATES).await?;

    println!("Vault initialised at {}", root.display());
    println!("  templates : {} created, {} preserved (existing files never overwritten)", tmpl_created, tmpl_skipped);
    println!("  db seed   : {} templates seeded, {} already present", db_created, db_skipped);
    println!("  rules     : {} created, {} preserved", rules_created, rules_skipped);
    println!("  anansi.toml: {}", toml_msg);
    Ok(()
    )
}

// ---------------------------------------------------------------------------
// cmd_ingest
// ---------------------------------------------------------------------------

async fn cmd_ingest(root: &Path, file: &Path) -> Result<()> {
    let config = Config::load(root)
        .with_context(|| format!("loading config from {}", root.display()))?;

    let db_pool = db::connect_and_migrate(&config.database_url).await?;
    let mut templates = TemplateRegistry::load(&config.templates_path(root))?;
    // Also load any user-created templates from DB
    let db_loaded = templates.load_from_db(&db_pool).await.unwrap_or(0);
    if db_loaded > 0 {
        eprintln!("[anansi2] loaded {db_loaded} template(s) from database");
    }
    let rules = RuleRegistry::load(&config.rules_path(root))?;
    let vault = Vault::new(root.to_path_buf(), &config.paths.web_dir);
    let llm = llm::build_client(&config.llm)?;

    let ctx = Arc::new(IngestContext {
        anansi_root: root.to_path_buf(),
        config,
        vault,
        db: db_pool,
        templates: tokio::sync::RwLock::new(templates),
        rules,
        llm: Some(llm),
    });

    let result = ingest(&ctx, file).await?;

    println!(
        "source_id={} outline_note_id={} created={} merged={} edges={} pass1_llm_called={}",
        result.source_id,
        result.outline_note_id,
        result.atomic_notes_created,
        result.atomic_notes_merged,
        result.edges_created,
        result.pass1_llm_called,
    );
    println!("duration_ms={}", result.duration_ms);

    Ok(())
}

// ---------------------------------------------------------------------------
// cmd_rebuild_wiki — disaster-recovery rebuild of the LLM-wiki from Postgres
// ---------------------------------------------------------------------------

async fn cmd_rebuild_wiki(root: &Path, clean: bool) -> Result<()> {
    let config = Config::load(root)
        .with_context(|| format!("loading config from {}", root.display()))?;

    eprintln!("[rebuild] connecting to db at {}", config.database_url);
    let pool = db::connect_and_migrate(&config.database_url).await?;

    // Force-enabled: a rebuild is an explicit operation, so it runs even when
    // [wiki] enabled = false. The configured dir and eviction caps still apply.
    let mut wiki = WikiStore::from_config(&config);
    wiki.enabled = true;
    eprintln!("[rebuild] wiki dir: {}", wiki.root.display());

    if clean {
        let removed = clean_wiki_dir(&wiki.root)?;
        eprintln!("[rebuild] cleaned {removed} existing wiki file(s)");
    }

    let report = wiki.crawl(&pool).await?;
    eprintln!(
        "[rebuild] done — {} resident, {} evicted, {} removed, {} errors",
        report.notes_projected, report.evicted, report.orphans_removed, report.errors
    );
    Ok(())
}

/// Remove only the wiki's own files (`*.md` + `.lint-state`) inside `dir`.
/// Never deletes the directory itself, other file types, or anything outside it.
/// Returns the count removed. A missing dir is not an error.
fn clean_wiki_dir(dir: &Path) -> Result<usize> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e).with_context(|| format!("reading wiki dir: {}", dir.display())),
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        // file_type() reads the dir entry and never follows symlinks, so we skip
        // directories AND symlinks — only ever unlinking real files we own.
        match entry.file_type() {
            Ok(ft) if ft.is_file() => {}
            _ => continue,
        }
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with(".md") || name == ".lint-state" {
            if let Err(e) = std::fs::remove_file(&path) {
                eprintln!("[rebuild] could not remove {}: {e}", path.display());
            } else {
                removed += 1;
            }
        }
    }
    Ok(removed)
}

// ---------------------------------------------------------------------------
// cmd_serve
// ---------------------------------------------------------------------------

async fn cmd_serve(root: &Path) -> Result<()> {
    eprintln!("[anansi2] serve starting — root={}", root.display());

    let config = Config::load(root)
        .with_context(|| format!("loading config from {}", root.display()))?;

    let host = config.server.host.clone();
    let port = config.server.mcp_port;
    eprintln!("[anansi2] config loaded — host={host} port={port} backend={}", config.llm.backend);

    eprintln!("[anansi2] connecting to db at {}", config.database_url);
    let db_pool = db::connect_and_migrate(&config.database_url).await?;

    let mut templates = TemplateRegistry::load(&config.templates_path(root)).unwrap_or_default();
    let rules = RuleRegistry::load(&config.rules_path(root)).unwrap_or_default();
    let vault = Vault::new(root.to_path_buf(), &config.paths.web_dir);

    // Seed any missing templates into DB (handles fresh DB without explicit init)
    let (db_seeded, _) = seed_templates_to_db(&db_pool, SEED_TEMPLATES).await
        .unwrap_or((0, 0));
    if db_seeded > 0 {
        eprintln!("[anansi2] seeded {db_seeded} template(s) into database");
    }

    // Load templates from DB — DB is the runtime source of truth and overrides
    // file-based templates. User-created templates via Claude Cowork appear here.
    let db_loaded = templates.load_from_db(&db_pool).await.unwrap_or(0);
    eprintln!("[anansi2] loaded {db_loaded} template(s) from database (runtime source of truth)");
    eprintln!("[anansi2] template registry: {} entity types total", templates.all_entity_types().len());

    // LLM not needed for MCP serve — atomized ingest is zero LLM calls.
    // Try to build one for legacy pipeline tools, but don't fail if unavailable.
    let llm = match llm::build_client(&config.llm) {
        Ok(client) => {
            eprintln!("[anansi2] llm client built ok");
            Some(client)
        }
        Err(e) => {
            eprintln!("[anansi2] llm client skipped: {e}");
            None
        }
    };

    let ctx = Arc::new(IngestContext {
        anansi_root: root.to_path_buf(),
        config,
        vault,
        db: db_pool,
        templates: tokio::sync::RwLock::new(templates),
        rules,
        llm,
    });

    eprintln!("[anansi2] binding to {host}:{port}");

    // Always spawn the queue watcher — it polls q-atomize/ for files dropped by
    // the inbox pipeline or anansi_ingest_atomized MCP tool.
    {
        let q_config = Arc::new(ctx.config.clone());
        let q_pool = ctx.db.clone();
        tokio::spawn(async move {
            queue::run_queue_watcher(q_config, q_pool).await;
        });
        eprintln!("[anansi2] queue watcher spawned — polling '{}' every {}s",
            ctx.config.inbox.queue_dir, ctx.config.inbox.queue_poll_interval_secs);
    }

    // Spawn inbox watcher if enabled
    if ctx.config.inbox.enabled {
        let watcher_config = Arc::new(ctx.config.clone());
        let watcher_pool = ctx.db.clone();
        tokio::spawn(async move {
            inbox::run_inbox_watcher(watcher_config, watcher_pool).await;
        });
        eprintln!("[anansi2] inbox watcher spawned — watching '{}'", ctx.config.inbox.watch_dir);
    } else {
        eprintln!("[anansi2] inbox watcher disabled (set inbox.enabled = true in anansi.toml to enable)");
    }

    // Spawn anansi-crawl watcher if the wiki and its crawl are both enabled
    if ctx.config.wiki.enabled && ctx.config.wiki.crawl_enabled {
        let c_config = Arc::new(ctx.config.clone());
        let c_pool = ctx.db.clone();
        tokio::spawn(async move {
            crawl::run_crawl_watcher(c_config, c_pool).await;
        });
        eprintln!("[anansi2] crawl watcher spawned — every {}s", ctx.config.wiki.crawl_interval_secs);
    } else {
        eprintln!("[anansi2] crawl watcher disabled (set wiki.enabled + wiki.crawl_enabled to enable)");
    }

    mcp::serve(ctx, &host, port).await?;
    Ok(())
}
