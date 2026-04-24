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

// Templates (16 files)
const TMPL_ACTION_ITEM_LIST: &str = include_str!("../templates/action_item_list.md");
const TMPL_AREA: &str = include_str!("../templates/area.md");
const TMPL_CONCEPT: &str = include_str!("../templates/concept.md");
const TMPL_CONTAINER: &str = include_str!("../templates/container.md");
const TMPL_CONTEXT: &str = include_str!("../templates/context.md");
const TMPL_EMAIL_THREAD: &str = include_str!("../templates/email_thread.md");
const TMPL_EVENT: &str = include_str!("../templates/event.md");
const TMPL_MEETING_SUMMARY: &str = include_str!("../templates/meeting_summary.md");
const TMPL_NOTE: &str = include_str!("../templates/note.md");
const TMPL_ORGANIZATION: &str = include_str!("../templates/organization.md");
const TMPL_OUTLINE: &str = include_str!("../templates/outline.md");
const TMPL_PERSON: &str = include_str!("../templates/person.md");
const TMPL_PROJECT: &str = include_str!("../templates/project.md");
const TMPL_RESEARCH_PAPER: &str = include_str!("../templates/research_paper.md");
const TMPL_TASK: &str = include_str!("../templates/task.md");
const TMPL_TOPIC: &str = include_str!("../templates/topic.md");

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
    // Create directory tree
    let dirs = ["web", "%Rules", "templates"];
    for dir in &dirs {
        let p = root.join(dir);
        std::fs::create_dir_all(&p)
            .with_context(|| format!("creating directory {}", p.display()))?;
    }

    // Seed template files (skip if already present)
    let templates: &[(&str, &str)] = &[
        ("action_item_list.md", TMPL_ACTION_ITEM_LIST),
        ("area.md", TMPL_AREA),
        ("concept.md", TMPL_CONCEPT),
        ("container.md", TMPL_CONTAINER),
        ("context.md", TMPL_CONTEXT),
        ("email_thread.md", TMPL_EMAIL_THREAD),
        ("event.md", TMPL_EVENT),
        ("meeting_summary.md", TMPL_MEETING_SUMMARY),
        ("note.md", TMPL_NOTE),
        ("organization.md", TMPL_ORGANIZATION),
        ("outline.md", TMPL_OUTLINE),
        ("person.md", TMPL_PERSON),
        ("project.md", TMPL_PROJECT),
        ("research_paper.md", TMPL_RESEARCH_PAPER),
        ("task.md", TMPL_TASK),
        ("topic.md", TMPL_TOPIC),
    ];
    for (name, content) in templates {
        let dest = root.join("templates").join(name);
        if !dest.exists() {
            std::fs::write(&dest, content)
                .with_context(|| format!("writing template {}", dest.display()))?;
        }
    }

    // Seed rules files (skip if already present)
    let rules: &[(&str, &str)] = &[
        ("%Atomicity.md", RULE_ATOMICITY),
        ("%Downstream-Flow.md", RULE_DOWNSTREAM_FLOW),
        ("%Merge-Strategy.md", RULE_MERGE_STRATEGY),
        ("%Template-Schema.md", RULE_TEMPLATE_SCHEMA),
    ];
    for (name, content) in rules {
        let dest = root.join("%Rules").join(name);
        if !dest.exists() {
            std::fs::write(&dest, content)
                .with_context(|| format!("writing rule {}", dest.display()))?;
        }
    }

    // Copy anansi.toml.example → anansi.toml (skip if already present)
    let toml_dest = root.join("anansi.toml");
    if !toml_dest.exists() {
        std::fs::write(&toml_dest, ANANSI_TOML_EXAMPLE)
            .with_context(|| format!("writing {}", toml_dest.display()))?;
    }

    // Open and migrate the database
    let config = Config::load(root)?;
    let db_path = config.db_path(root);
    db::open_and_migrate(&db_path).await
        .with_context(|| format!("opening/migrating DB at {}", db_path.display()))?;

    println!("Vault initialised at {}", root.display());
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

    mcp::serve(ctx, &host, port).await?;
    Ok(())
}
