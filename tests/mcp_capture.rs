use std::path::{Path, PathBuf};
use std::sync::Arc;

use anansi2::config::{
    Config, InboxConfig, InferSettings, LlmConfig, PathsConfig, PipelineConfig, ServerConfig,
    WikiConfig,
};
use anansi2::db::{self, connect_and_migrate, NoteRecord};
use anansi2::pipeline::IngestContext;
use anansi2::rules::RuleRegistry;
use anansi2::template::TemplateRegistry;
use anansi2::vault::Vault;
use serde_json::{json, Value};
use uuid::Uuid;

/// Build an IngestContext with the wiki disabled (so materialize is a no-op)
/// and no LLM (tool_capture / tool_edges don't need one). Skips cleanly when
/// DATABASE_URL is unset — mirrors tests/pipeline_integration.rs.
async fn make_capture_context(root: &Path) -> Option<IngestContext> {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("[skip] DATABASE_URL not set — skipping integration test");
            return None;
        }
    };

    let web_dir = root.join("web");
    std::fs::create_dir_all(&web_dir).unwrap();

    let db = connect_and_migrate(&database_url).await.unwrap();
    let vault = Vault::new(root.to_path_buf(), Path::new("web"));

    let config = Config {
        paths: PathsConfig {
            web_dir: PathBuf::from("web"),
            rules_dir: PathBuf::from("%Rules"),
            templates_dir: PathBuf::from("templates"),
        },
        llm: LlmConfig {
            backend: "ollama".to_string(),
            url: "http://localhost:11434".to_string(),
            model: "qwen2.5:14b".to_string(),
            n_ctx: 16384,
            timeout_s: 60,
            decomposition: InferSettings {
                temperature: 0.2,
                max_tokens: 4096,
                json_mode: false,
            },
            extraction: InferSettings {
                temperature: 0.2,
                max_tokens: 4096,
                json_mode: true,
            },
            synthesis: InferSettings {
                temperature: 0.2,
                max_tokens: 4096,
                json_mode: true,
            },
            gemini: None,
            openrouter: None,
        },
        server: ServerConfig::default(),
        pipeline: PipelineConfig { mode: None },
        inbox: InboxConfig::default(),
        wiki: WikiConfig::default(),
        tender: anansi2::config::TenderConfig {
            enabled: false,
            interval_secs: 3600,
            batch_size: 100,
            dry_run: true,
        },
        database_url,
    };

    Some(IngestContext {
        anansi_root: root.to_path_buf(),
        config,
        vault,
        db,
        templates: tokio::sync::RwLock::new(TemplateRegistry::default()),
        rules: RuleRegistry::default(),
        llm: None,
    })
}

/// Parse the JSON-RPC `result.content[0].text` payload returned by the tools.
fn parse_text_payload(resp: &Value) -> Value {
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("response should carry a text payload");
    serde_json::from_str(text).expect("text payload should be valid JSON")
}

#[tokio::test]
async fn test_capture_with_attach_to_outline_creates_edge_and_appends_content() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let ctx = match make_capture_context(root).await {
        Some(c) => c,
        None => return, // DATABASE_URL not set, skip
    };
    let state = anansi2::mcp::McpState {
        ctx: Arc::new(ctx),
        api_key: None,
    };

    // Unique per-run keys so repeated runs against a shared DB stay deterministic.
    let suffix = Uuid::new_v4().to_string();
    let outline_key = format!("outline:test-offload-{suffix}");
    let outline_id = Uuid::new_v4().to_string();

    // Seed the outline note.
    db::insert_note(
        &state.ctx.db,
        &NoteRecord {
            id: outline_id.clone(),
            entity_type: "outline".to_string(),
            name: format!("Test Offload {suffix}"),
            match_key: outline_key.clone(),
            lede: Some("Test outline.".to_string()),
            why: None,
            content: Some("Initial outline.".to_string()),
            has_conflicts: 0,
            conflicts_updated_at: None,
            merge_category: "entity".to_string(),
            created_from: db::MANUAL_SOURCE_ID.to_string(),
            source_count: 1,
            created_at: db::now_rfc3339(),
            updated_at: db::now_rfc3339(),
        },
    )
    .await
    .unwrap();

    let resp = anansi2::mcp::tool_capture(
        state.clone(),
        json!(1),
        json!({
            "entity_type": "note",
            "name": format!("My Fact {suffix}"),
            "lede": "A fact.",
            "source": "skill",
            "attach_to_outline": outline_key,
        }),
    )
    .await
    .0;
    let parsed = parse_text_payload(&resp);
    assert_eq!(parsed["status"].as_str(), Some("created"));
    assert_eq!(parsed["attached_to"].as_str(), Some(outline_key.as_str()));
    let new_note_id = parsed["note_id"].as_str().unwrap().to_string();
    let new_match_key = parsed["match_key"].as_str().unwrap().to_string();

    // The part_of edge exists: captured note -> outline, manual sentinel source.
    let edges = db::edges_for_note(&state.ctx.db, &outline_id).await.unwrap();
    let part_of = edges.iter().find(|e| {
        e.edge_type == "part_of"
            && e.source_note_id == new_note_id
            && e.target_note_id == outline_id
            && e.from_source == db::MANUAL_SOURCE_ID
    });
    assert!(part_of.is_some(), "expected part_of edge from new note to outline");

    // The outline content got the match_key appended.
    let outline_after = db::find_note_by_match_key(&state.ctx.db, &outline_key)
        .await
        .unwrap()
        .expect("outline should still exist");
    assert_eq!(
        outline_after.content.as_deref(),
        Some(format!("Initial outline.\n{new_match_key}").as_str())
    );
}

#[tokio::test]
async fn test_capture_without_attach_is_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let ctx = match make_capture_context(root).await {
        Some(c) => c,
        None => return, // DATABASE_URL not set, skip
    };
    let state = anansi2::mcp::McpState {
        ctx: Arc::new(ctx),
        api_key: None,
    };

    let suffix = Uuid::new_v4().to_string();
    let resp = anansi2::mcp::tool_capture(
        state.clone(),
        json!(1),
        json!({
            "entity_type": "note",
            "name": format!("Bare Fact {suffix}"),
            "lede": "Bare.",
            "source": "skill",
        }),
    )
    .await
    .0;
    let parsed = parse_text_payload(&resp);
    assert_eq!(parsed["status"].as_str(), Some("created"));
    assert!(parsed["attached_to"].is_null(), "attached_to should be null when omitted");

    let new_note_id = parsed["note_id"].as_str().unwrap().to_string();
    let edges = db::edges_for_note(&state.ctx.db, &new_note_id).await.unwrap();
    assert!(
        !edges.iter().any(|e| e.edge_type == "part_of"),
        "no part_of edges expected without attach_to_outline"
    );
}

#[tokio::test]
async fn test_anansi_edges_outline_part_of_returns_new_note() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let ctx = match make_capture_context(root).await {
        Some(c) => c,
        None => return, // DATABASE_URL not set, skip
    };
    let state = anansi2::mcp::McpState {
        ctx: Arc::new(ctx),
        api_key: None,
    };

    let suffix = Uuid::new_v4().to_string();
    let outline_key = format!("outline:test-offload-{suffix}");
    let outline_id = Uuid::new_v4().to_string();

    db::insert_note(
        &state.ctx.db,
        &NoteRecord {
            id: outline_id.clone(),
            entity_type: "outline".to_string(),
            name: format!("Test Offload {suffix}"),
            match_key: outline_key.clone(),
            lede: Some("Test outline.".to_string()),
            why: None,
            content: Some("Initial outline.".to_string()),
            has_conflicts: 0,
            conflicts_updated_at: None,
            merge_category: "entity".to_string(),
            created_from: db::MANUAL_SOURCE_ID.to_string(),
            source_count: 1,
            created_at: db::now_rfc3339(),
            updated_at: db::now_rfc3339(),
        },
    )
    .await
    .unwrap();

    let resp = anansi2::mcp::tool_capture(
        state.clone(),
        json!(1),
        json!({
            "entity_type": "note",
            "name": format!("My Fact {suffix}"),
            "lede": "A fact.",
            "source": "skill",
            "attach_to_outline": outline_key,
        }),
    )
    .await
    .0;
    let parsed = parse_text_payload(&resp);
    let new_note_id = parsed["note_id"].as_str().unwrap().to_string();

    // anansi_edges on the outline with edge_type=part_of discovers the note.
    let resp = anansi2::mcp::tool_edges(
        state,
        json!(2),
        json!({
            "id": outline_id,
            "depth": 1,
            "edge_type": "part_of",
        }),
    )
    .await
    .0;
    let parsed = parse_text_payload(&resp);
    let nodes = parsed["nodes"].as_array().expect("nodes should be an array");
    assert!(
        nodes.iter().any(|n| n["id"].as_str() == Some(new_note_id.as_str())),
        "expected captured note in anansi_edges nodes, got {nodes:?}"
    );
}
