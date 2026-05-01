use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use anansi2::config::Config;
use anansi2::db;
use anansi2::llm;
use anansi2::mcp;
use anansi2::pipeline::{ingest, IngestContext};
use anansi2::rules::RuleRegistry;
use anansi2::template::TemplateRegistry;
use anansi2::vault::Vault;

// ---------------------------------------------------------------------------
// Embedded seed files (compile-time include_str!)
// ---------------------------------------------------------------------------

// Templates (21 files)
const TMPL_ACTION_ITEM_LIST: &str = include_str!("../templates/action_item_list.md");
const TMPL_CONTAINER: &str = include_str!("../templates/container.md");
const TMPL_CONTEXT: &str = include_str!("../templates/context.md");
const TMPL_EVENT: &str = include_str!("../templates/event.md");
const TMPL_OUTLINE: &str = include_str!("../templates/outline.md");
const TMPL_TASK: &str = include_str!("../templates/task.md");
// entity-* templates
const TMPL_AREA: &str = include_str!("../templates/entity-area.md");
const TMPL_NOTE: &str = include_str!("../templates/entity-note.md");
const TMPL_ORGANIZATION: &str = include_str!("../templates/entity-organization.md");
const TMPL_PERSON: &str = include_str!("../templates/entity-person.md");
const TMPL_PROJECT: &str = include_str!("../templates/entity-project.md");
const TMPL_TOPIC: &str = include_str!("../templates/identity-topic.md");
// meeting family
const TMPL_MEETING_SUMMARY: &str = include_str!("../templates/meeting-summary.md");
const TMPL_MEETING_TOPIC_DISCUSSION: &str = include_str!("../templates/meeting-topic-discussion.md");
// research family
const TMPL_RESEARCH_PAPER: &str = include_str!("../templates/research-paper.md");
const TMPL_RESEARCH_SECTION: &str = include_str!("../templates/research-section.md");
// youtube family
const TMPL_YOUTUBE_VIDEO: &str = include_str!("../templates/youtube-video.md");
const TMPL_YOUTUBE_CHAPTER: &str = include_str!("../templates/youtube-chapter.md");
// email family
const TMPL_EMAIL_THREAD: &str = include_str!("../templates/email-thread.md");
const TMPL_EMAIL_EXCHANGE: &str = include_str!("../templates/email-exchange.md");

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
    }
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
    let templates: &[(&str, &str)] = &[
        ("action_item_list.md", TMPL_ACTION_ITEM_LIST),
        ("container.md", TMPL_CONTAINER),
        ("context.md", TMPL_CONTEXT),
        ("event.md", TMPL_EVENT),
        ("outline.md", TMPL_OUTLINE),
        ("task.md", TMPL_TASK),
        ("entity-area.md", TMPL_AREA),
        ("entity-note.md", TMPL_NOTE),
        ("entity-organization.md", TMPL_ORGANIZATION),
        ("entity-person.md", TMPL_PERSON),
        ("entity-project.md", TMPL_PROJECT),
        ("identity-topic.md", TMPL_TOPIC),
        ("meeting-summary.md", TMPL_MEETING_SUMMARY),
        ("meeting-topic-discussion.md", TMPL_MEETING_TOPIC_DISCUSSION),
        ("research-paper.md", TMPL_RESEARCH_PAPER),
        ("research-section.md", TMPL_RESEARCH_SECTION),
        ("youtube-video.md", TMPL_YOUTUBE_VIDEO),
        ("youtube-chapter.md", TMPL_YOUTUBE_CHAPTER),
        ("email-thread.md", TMPL_EMAIL_THREAD),
        ("email-exchange.md", TMPL_EMAIL_EXCHANGE),
    ];
    let mut tmpl_created = 0u32;
    let mut tmpl_skipped = 0u32;
    for (name, content) in templates {
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
    let db_path = config.db_path(root);
    db::open_and_migrate(&db_path).await
        .with_context(|| format!("opening/migrating DB at {}", db_path.display()))?;

    println!("Vault initialised at {}", root.display());
    println!("  templates : {} created, {} preserved (existing files never overwritten)", tmpl_created, tmpl_skipped);
    println!("  rules     : {} created, {} preserved", rules_created, rules_skipped);
    println!("  anansi.toml: {}", toml_msg);
    Ok(())
}

// ---------------------------------------------------------------------------
// cmd_ingest
// ---------------------------------------------------------------------------

async fn cmd_ingest(root: &Path, file: &Path) -> Result<()> {
    let config = Config::load(root)
        .with_context(|| format!("loading config from {}", root.display()))?;

    let db_pool = db::open_and_migrate(&config.db_path(root)).await?;
    let templates = TemplateRegistry::load(&config.templates_path(root))?;
    let rules = RuleRegistry::load(&config.rules_path(root))?;
    let vault = Vault::new(root.to_path_buf(), &config.paths.web_dir);
    let llm = llm::build_client(&config.llm)?;

    let ctx = Arc::new(IngestContext {
        anansi_root: root.to_path_buf(),
        config,
        vault,
        db: db_pool,
        templates,
        rules,
        llm,
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
// cmd_serve
// ---------------------------------------------------------------------------

async fn cmd_serve(root: &Path) -> Result<()> {
    let config = Config::load(root)
        .with_context(|| format!("loading config from {}", root.display()))?;

    let host = config.server.host.clone();
    let port = config.server.mcp_port;

    // Ensure the anansi subdirectory exists for the DB (vault may be empty on first boot)
    let db_path = config.db_path(root);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating db directory {}", parent.display()))?;
    }

    let db_pool = db::open_and_migrate(&db_path).await?;
    let templates = TemplateRegistry::load(&config.templates_path(root)).unwrap_or_default();
    let rules = RuleRegistry::load(&config.rules_path(root)).unwrap_or_default();
    let vault = Vault::new(root.to_path_buf(), &config.paths.web_dir);
    let llm = llm::build_client(&config.llm)?;

    let ctx = Arc::new(IngestContext {
        anansi_root: root.to_path_buf(),
        config,
        vault,
        db: db_pool,
        templates,
        rules,
        llm,
    });

    mcp::serve(ctx, &host, port).await?;
    Ok(())
}
