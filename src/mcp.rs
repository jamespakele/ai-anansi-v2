use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;


use crate::db::{self, EdgeRecord, now_rfc3339};
use crate::embed;
use crate::export;
use crate::pipeline::{ingest, IngestContext};

// ---------------------------------------------------------------------------
// MCP State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct McpState {
    pub ctx: Arc<IngestContext>,
}

// ---------------------------------------------------------------------------
// JSON-RPC types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub id: Value,
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

fn json_rpc_ok(id: Value, result: Value) -> Json<Value> {
    Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    }))
}

fn json_rpc_err(id: Value, code: i32, message: &str) -> Json<Value> {
    Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message,
        },
    }))
}

/// Reject calls that didn't come through the anansi skill.
/// The skill passes `source: "skill"`; direct MCP calls default to `"mcp"`.
/// Returns None if the call is allowed, or an error Json response if not.
fn check_skill_source(id: &Value, args: &Value) -> Option<Json<Value>> {
    let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("mcp");
    if source == "skill" {
        None
    } else {
        Some(json_rpc_err(
            id.clone(),
            -32000,
            "Direct MCP calls not permitted. Route through the anansi skill.",
        ))
    }
}

// ---------------------------------------------------------------------------
// Router / serve
// ---------------------------------------------------------------------------

pub fn router(ctx: Arc<IngestContext>) -> Router {
    Router::new()
        .route("/", post(handle_json_rpc))
        .route("/health", get(handle_health))
        .route("/exports/{filename}", get(handle_export_download))
        .route("/upload/inbox",   post(handle_upload_inbox))
        .route("/upload/atomize", post(handle_upload_atomize))
        .with_state(McpState { ctx })
}

pub async fn serve(ctx: Arc<IngestContext>, host: &str, port: u16) -> Result<()> {
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("MCP server listening on {addr}");
    axum::serve(listener, router(ctx)).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// JSON-RPC dispatcher
// ---------------------------------------------------------------------------

async fn handle_json_rpc(
    State(state): State<McpState>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<Value> {
    match req.method.as_str() {
        "initialize" => handle_initialize(req.id),
        "tools/list" => handle_tools_list(req.id),
        "tools/call" => handle_tools_call(state, req.id, req.params).await,
        _ => json_rpc_err(req.id, -32601, "Method not found"),
    }
}

// ---------------------------------------------------------------------------
// initialize
// ---------------------------------------------------------------------------

fn handle_initialize(id: Value) -> Json<Value> {
    json_rpc_ok(
        id,
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "anansi2", "version": "0.1.0" },
        }),
    )
}

// ---------------------------------------------------------------------------
// tools/list
// ---------------------------------------------------------------------------

fn handle_tools_list(id: Value) -> Json<Value> {
    json_rpc_ok(
        id,
        json!({
            "tools": [
                {
                    "name": "anansi_search",
                    "description": "Full-text search across note name, lede, why, and content using PostgreSQL tsvector. Supports plain-text queries. Use for keyword and concept lookups. Archived notes (entity_type starting with 'archive-') are excluded by default.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Search query. Plain terms or phrases." },
                            "limit": { "type": "integer", "description": "Max results (default 20)." },
                            "include_archived": { "type": "boolean", "description": "If true, include archived notes in results. Default false." }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "anansi_get",
                    "description": "Return a note by id or match_key, including file content if present.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Note UUID." },
                            "match_key": { "type": "string", "description": "Note match_key (entity_type:slug)." }
                        }
                    }
                },
                {
                    "name": "anansi_filter",
                    "description": "Filter notes by entity_type and/or date range. All parameters optional. Archived notes (entity_type starting with 'archive-') are excluded by default when no entity_type is specified. Use entity_type='archive-discussion' to retrieve archived notes of a specific type.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "entity_type": { "type": "string", "description": "Filter by type, e.g. person, organization, project, event, topic, note. Use 'archive-<type>' to retrieve archived notes." },
                            "after": { "type": "string", "description": "ISO 8601 datetime — return notes updated at or after this timestamp." },
                            "before": { "type": "string", "description": "ISO 8601 datetime — return notes updated at or before this timestamp." },
                            "limit": { "type": "integer", "description": "Max results (default 50)." },
                            "include_archived": { "type": "boolean", "description": "If true, include archived notes when no entity_type filter is set. Default false." }
                        }
                    }
                },
                {
                    "name": "anansi_edges",
                    "description": "BFS traversal from a note. Returns connected notes and edge metadata.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Starting note UUID." },
                            "depth": { "type": "integer", "description": "BFS depth 1–5 (default 1)." },
                            "edge_type": { "type": "string", "description": "Optional edge type filter." }
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "anansi_relate",
                    "description": "Manually declare an edge between two notes. Gated behind read_only.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "source_id": { "type": "string", "description": "Source note UUID." },
                            "target_id": { "type": "string", "description": "Target note UUID." },
                            "edge_type": { "type": "string", "description": "Relationship type label." },
                            "why": { "type": "string", "description": "Optional reason for the edge." },
                            "source": { "type": "string", "default": "mcp", "description": "Call source. Set to 'skill' by the anansi skill — do not override." }
                        },
                        "required": ["source_id", "target_id", "edge_type"]
                    }
                },
                {
                    "name": "anansi_ingest_atomized",
                    "description": "Ingest an atomized block set from the atomize or smart-brevity skill. Pass the atomized content directly (preferred — avoids disk write) or a file path. Optionally pass the PARA TOC from Pass 1 to store as the outline note content and source record. Creates notes with lede/why/content, an outline note for document recomposition, and hierarchy edges.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "The full atomized blocks text (<!-- anansi-atomize: ... --> through <!-- concepts: ... -->). Preferred — pass the in-memory Pass 2 output directly."
                            },
                            "path": {
                                "type": "string",
                                "description": "Absolute path to a saved atomized .md file. Used only if content is not provided."
                            },
                            "para_toc": {
                                "type": "string",
                                "description": "Optional. The typed PARA TOC output from Pass 1 of the atomize skill. Stored as the source record's toc_text and as the outline note's content."
                            },
                            "source_path": {
                                "type": "string",
                                "description": "Optional. Path to the original source document being atomized (for source record attribution)."
                            },
                            "source": { "type": "string", "default": "mcp", "description": "Call source. Set to 'skill' by the anansi skill — do not override." }
                        }
                    }
                },
                {
                    "name": "anansi_ingest_file",
                    "description": "Drop a raw document into the inbox pipeline (q-inbox/). The server runs the full skill pipeline: para-projects-areas + para-resource-entities in parallel, then sb-atomize, then ingest into the knowledge base. Use this when you have a source document (meeting notes, book chapter, article, etc.) that needs full LLM processing. For already-atomized content use anansi_ingest_atomized instead.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "The raw document text to process."
                            },
                            "filename": {
                                "type": "string",
                                "description": "Optional filename for the queued file (e.g. 'smart-brevity-chapter-3.md'). If omitted a slug+timestamp name is generated automatically."
                            },
                            "source": { "type": "string", "default": "mcp", "description": "Call source. Set to 'skill' by the anansi skill — do not override." }
                        },
                        "required": ["content"]
                    }
                },
                {
                    "name": "anansi_capture",
                    "description": "Quick-capture a single note (person, event, organization, topic, etc.) without atomization. Upserts by match_key — safe to call multiple times for the same entity.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "entity_type": { "type": "string", "description": "Type tag: person, organization, event, topic, note, project, etc." },
                            "name": { "type": "string", "description": "Display name, e.g. 'John Doe' or 'Meeting with Lynn'." },
                            "lede": { "type": "string", "description": "The single most important fact about this entity." },
                            "why": { "type": "string", "description": "Optional. Why this entity matters — one sentence of context." },
                            "content": { "type": "string", "description": "Optional. Additional details, bullet points, structured info." },
                            "source": { "type": "string", "default": "mcp", "description": "Call source. Set to 'skill' by the anansi skill — do not override." }
                        },
                        "required": ["entity_type", "name", "lede"]
                    }
                },
                {
                    "name": "anansi_purge",
                    "description": "Delete all notes, edges, and contributions from a specific import by source_id. Removes the source record too. Use anansi_search or anansi_filter to find source_ids.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "source_id": { "type": "string", "description": "UUID of the source record to purge." },
                            "source": { "type": "string", "default": "mcp", "description": "Call source. Set to 'skill' by the anansi skill — do not override." }
                        },
                        "required": ["source_id"]
                    }
                },
                {
                    "name": "anansi_delete_note",
                    "description": "Delete a single note by its id. Also removes all edges connected to that note (both directions) and its source_contribution records. Embeddings are removed automatically via cascade. Use anansi_get or anansi_search to find the note id first.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "note_id": { "type": "string", "description": "ID of the note to delete." },
                            "source": { "type": "string", "default": "mcp", "description": "Call source. Set to 'skill' by the anansi skill — do not override." }
                        },
                        "required": ["note_id"]
                    }
                },
                {
                    "name": "anansi_archive_note",
                    "description": "Soft-archive a note by prefixing its entity_type with 'archive-' (e.g. 'discussion' → 'archive-discussion'). Non-destructive — edges, embeddings, and content are preserved. Pass restore:true to reverse. Archived notes are excluded from normal searches but can be found with anansi_filter using entity_type='archive-<type>'.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "note_id": { "type": "string", "description": "ID of the note to archive or restore." },
                            "restore": { "type": "boolean", "description": "If true, remove the 'archive-' prefix to restore the note. Default false." },
                            "source": { "type": "string", "default": "mcp", "description": "Call source. Set to 'skill' by the anansi skill — do not override." }
                        },
                        "required": ["note_id"]
                    }
                },
                {
                    "name": "anansi_embed",
                    "description": "Generate and store Gemini text-embedding-004 embeddings for notes. Pass note_id for a single note, or batch:true to embed up to 100 notes that don't yet have embeddings.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "note_id": { "type": "string", "description": "UUID of a specific note to embed." },
                            "batch": { "type": "boolean", "description": "If true, embed up to 100 un-embedded notes." }
                        }
                    }
                },
                {
                    "name": "anansi_search_semantic",
                    "description": "Semantic similarity search using vector embeddings. Returns notes most similar to the query text by cosine distance. Requires notes to have embeddings (use anansi_embed first).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Natural-language query to find similar notes." },
                            "limit": { "type": "integer", "description": "Max results (default 10)." },
                            "model": { "type": "string", "description": "Embedding model name (default: text-embedding-004)." }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "anansi_export_context",
                    "description": "BFS graph traversal from a note, composing all connected notes into a single flat markdown document optimized for LLM consumption. Returns the full context inline.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Root note UUID to start BFS from." },
                            "depth": { "type": "integer", "description": "BFS depth 1–5 (default 2)." }
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "anansi_export_vault",
                    "description": "BFS graph traversal from a note, exporting all connected notes as an Obsidian-compatible vault zip. Returns a download URL.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Root note UUID to start BFS from." },
                            "depth": { "type": "integer", "description": "BFS depth 1–5 (default 2)." }
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "anansi_get_upload_url",
                    "description": "Returns the HTTP upload URLs for this anansi server (inbox and atomize queues). Use the returned curl command to upload a local file directly to the server — faster than passing file content through MCP. Requires bash/terminal access (available in Claude Code).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "local_path": {
                                "type": "string",
                                "description": "Optional. Local file path to embed in the returned curl command, e.g. '/home/user/my-doc.md'."
                            }
                        }
                    }
                }
            ]
        }),
    )
}

// ---------------------------------------------------------------------------
// tools/call dispatcher
// ---------------------------------------------------------------------------

async fn handle_tools_call(state: McpState, id: Value, params: Option<Value>) -> Json<Value> {
    let params = match params {
        Some(p) => p,
        None => return json_rpc_err(id, -32602, "Missing params"),
    };

    let tool_name = match params.get("name").and_then(|n| n.as_str()) {
        Some(n) => n.to_string(),
        None => return json_rpc_err(id, -32602, "Missing params.name"),
    };

    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match tool_name.as_str() {
        "anansi_ingest" => json_rpc_err(id, -32000, "anansi_ingest is disabled. Use anansi_ingest_atomized instead."),
        "anansi_search" => tool_search(state, id, args).await,
        "anansi_get" => tool_get(state, id, args).await,
        "anansi_filter" => tool_filter(state, id, args).await,
        "anansi_edges" => tool_edges(state, id, args).await,
        "anansi_relate" => tool_relate(state, id, args).await,
        "anansi_get_upload_url" => tool_get_upload_url(state, id, args).await,
        "anansi_ingest_atomized" => tool_ingest_atomized(state, id, args).await,
        "anansi_ingest_file"     => tool_ingest_file(state, id, args).await,
        "anansi_capture" => tool_capture(state, id, args).await,
        "anansi_purge" => tool_purge(state, id, args).await,
        "anansi_delete_note" => tool_delete_note(state, id, args).await,
        "anansi_archive_note" => tool_archive_note(state, id, args).await,
        "anansi_embed" => tool_embed(state, id, args).await,
        "anansi_search_semantic" => tool_search_semantic(state, id, args).await,
        "anansi_export_context" => tool_export_context(state, id, args).await,
        "anansi_export_vault" => tool_export_vault(state, id, args).await,
        other => json_rpc_err(id, -32601, &format!("Unknown tool: {other}")),
    }
}

// ---------------------------------------------------------------------------
// anansi_ingest
// ---------------------------------------------------------------------------

async fn tool_ingest(state: McpState, id: Value, args: Value) -> Json<Value> {
    let ctx = &state.ctx;

    if ctx.config.server.read_only {
        return json_rpc_err(id, -32000, "this anansi instance is read-only");
    }

    // Resolve the file path
    let source_path = if let Some(sp) = args.get("source_path").and_then(|v| v.as_str()) {
        std::path::PathBuf::from(sp)
    } else if let (Some(content), Some(filename)) = (
        args.get("content").and_then(|v| v.as_str()),
        args.get("filename").and_then(|v| v.as_str()),
    ) {
        // Validate filename — reject any path traversal attempt
        if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
            return json_rpc_err(id, -32602, "filename must not contain path separators or '..'");
        }
        let dest = ctx.anansi_root.join(filename);
        if let Err(e) = tokio::fs::write(&dest, content).await {
            return json_rpc_err(id, -32000, &format!("Failed to write file: {e}"));
        }
        dest
    } else {
        return json_rpc_err(
            id,
            -32602,
            "Provide source_path, or both content and filename",
        );
    };

    let ctx_bg = Arc::clone(ctx);
    let path_bg = source_path.clone();
    tokio::spawn(async move {
        match ingest(&ctx_bg, &path_bg).await {
            Ok(r) => eprintln!("INFO: ingest complete: source_id={} notes_created={} duration_ms={}", r.source_id, r.atomic_notes_created, r.duration_ms),
            Err(e) => eprintln!("ERROR: ingest failed for {}: {e:#}", path_bg.display()),
        }
    });

    json_rpc_ok(
        id,
        json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&json!({
                    "status": "queued",
                    "source_path": source_path.to_string_lossy(),
                    "message": "Ingest started in background. Use anansi_search to check results in a few minutes."
                })).unwrap_or_default()
            }]
        }),
    )
}

// ---------------------------------------------------------------------------
// anansi_search
// ---------------------------------------------------------------------------

async fn tool_search(state: McpState, id: Value, args: Value) -> Json<Value> {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: query"),
    };
    let limit = args
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(20)
        .clamp(1, 1000);

    let include_archived = args
        .get("include_archived")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match db::search_notes(&state.ctx.db, &query, limit, include_archived).await {
        Ok(notes) => {
            let items: Vec<Value> = notes
                .into_iter()
                .map(|n| {
                    json!({
                        "id": n.id,
                        "name": n.name,
                        "entity_type": n.entity_type,
                        "match_key": n.match_key,
                        "lede": n.lede,
                    })
                })
                .collect();
            json_rpc_ok(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&items).unwrap_or_default()
                    }]
                }),
            )
        }
        Err(e) => json_rpc_err(id, -32000, &format!("Search failed: {e}")),
    }
}

// ---------------------------------------------------------------------------
// anansi_filter
// ---------------------------------------------------------------------------

async fn tool_filter(state: McpState, id: Value, args: Value) -> Json<Value> {
    let entity_type = args.get("entity_type").and_then(|v| v.as_str());
    let after       = args.get("after").and_then(|v| v.as_str());
    let before      = args.get("before").and_then(|v| v.as_str());
    let limit = args
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(50)
        .clamp(1, 1000);

    let include_archived = args
        .get("include_archived")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match db::filter_notes(&state.ctx.db, entity_type, after, before, limit, include_archived).await {
        Ok(notes) => {
            let items: Vec<Value> = notes
                .into_iter()
                .map(|n| json!({
                    "id": n.id,
                    "name": n.name,
                    "entity_type": n.entity_type,
                    "match_key": n.match_key,
                    "lede": n.lede,
                    "updated_at": n.updated_at,
                }))
                .collect();
            json_rpc_ok(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&items).unwrap_or_default()
                    }]
                }),
            )
        }
        Err(e) => json_rpc_err(id, -32000, &format!("Filter failed: {e}")),
    }
}

// ---------------------------------------------------------------------------
// anansi_get
// ---------------------------------------------------------------------------

async fn tool_get(state: McpState, id: Value, args: Value) -> Json<Value> {
    let note_result = if let Some(note_id) = args.get("id").and_then(|v| v.as_str()) {
        db::get_note(&state.ctx.db, note_id).await
    } else if let Some(mk) = args.get("match_key").and_then(|v| v.as_str()) {
        db::find_note_by_match_key(&state.ctx.db, mk).await
    } else {
        return json_rpc_err(id, -32602, "Provide id or match_key");
    };

    match note_result {
        Err(e) => json_rpc_err(id, -32000, &format!("DB error: {e}")),
        Ok(None) => json_rpc_err(id, -32000, "Note not found"),
        Ok(Some(note)) => {
            // Count edges
            let edge_count = db::edges_for_note(&state.ctx.db, &note.id)
                .await
                .map(|edges| edges.len())
                .unwrap_or(0);

            let mut result = json!({
                "id": note.id,
                "entity_type": note.entity_type,
                "name": note.name,
                "match_key": note.match_key,
                "lede": note.lede,
                "why": note.why,
                "content": note.content,
                "created_from": note.created_from,
                "has_conflicts": note.has_conflicts,
                "conflicts_updated_at": note.conflicts_updated_at,
                "merge_category": note.merge_category,
                "source_count": note.source_count,
                "created_at": note.created_at,
                "updated_at": note.updated_at,
                "edge_count": edge_count,
            });

            if note.content.is_none() {
                result["warning"] = Value::String("content_not_available".to_string());
                result["message"] = Value::String("note has no inline content yet".to_string());
            }

            json_rpc_ok(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&result).unwrap_or_default()
                    }]
                }),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// anansi_edges  (BFS via edges_for_note + visited HashSet, depth 1–5)
// ---------------------------------------------------------------------------

async fn tool_edges(state: McpState, id: Value, args: Value) -> Json<Value> {
    let start_id = match args.get("id").and_then(|v| v.as_str()) {
        Some(i) => i.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: id"),
    };

    let depth = args
        .get("depth")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .clamp(1, 5) as usize;

    let edge_type_filter = args
        .get("edge_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let pool = &state.ctx.db;

    // BFS
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut frontier: Vec<String> = vec![start_id.clone()];
    visited.insert(start_id.clone());

    let mut all_edges: Vec<Value> = Vec::new();
    let mut edge_ids_seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut all_nodes: std::collections::HashMap<String, Value> = std::collections::HashMap::new();

    for _level in 0..depth {
        if frontier.is_empty() {
            break;
        }
        let mut next_frontier: Vec<String> = Vec::new();

        for node_id in &frontier {
            let edges = match db::edges_for_note(pool, node_id).await {
                Ok(e) => e,
                Err(e) => return json_rpc_err(id, -32000, &format!("DB error: {e}")),
            };

            for edge in edges {
                // Apply optional edge_type filter
                if let Some(ref et) = edge_type_filter {
                    if &edge.edge_type != et {
                        continue;
                    }
                }

                // Identify the neighbour
                let neighbour_id = if edge.source_note_id == *node_id {
                    edge.target_note_id.clone()
                } else {
                    edge.source_note_id.clone()
                };

                if edge_ids_seen.insert(edge.id.clone()) {
                    all_edges.push(json!({
                        "id": edge.id,
                        "source_note_id": edge.source_note_id,
                        "target_note_id": edge.target_note_id,
                        "edge_type": edge.edge_type,
                        "why": edge.why,
                        "weight": edge.weight,
                    }));
                }

                if !visited.contains(&neighbour_id) {
                    visited.insert(neighbour_id.clone());
                    next_frontier.push(neighbour_id.clone());

                    // Fetch note info for the neighbour
                    if let Ok(Some(note)) = db::get_note(pool, &neighbour_id).await {
                        all_nodes.insert(
                            neighbour_id.clone(),
                            json!({
                                "id": note.id,
                                "name": note.name,
                                "entity_type": note.entity_type,
                                "match_key": note.match_key,
                            }),
                        );
                    }
                }
            }
        }

        frontier = next_frontier;
    }

    let nodes_vec: Vec<Value> = all_nodes.into_values().collect();

    json_rpc_ok(
        id,
        json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&json!({
                    "start_id": start_id,
                    "depth": depth,
                    "nodes": nodes_vec,
                    "edges": all_edges,
                })).unwrap_or_default()
            }]
        }),
    )
}

// ---------------------------------------------------------------------------
// anansi_relate
// ---------------------------------------------------------------------------

async fn tool_relate(state: McpState, id: Value, args: Value) -> Json<Value> {
    let ctx = &state.ctx;

    if let Some(err) = check_skill_source(&id, &args) { return err; }

    if ctx.config.server.read_only {
        return json_rpc_err(id, -32000, "this anansi instance is read-only");
    }

    let source_id = match args.get("source_id").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: source_id"),
    };
    let target_id = match args.get("target_id").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: target_id"),
    };
    let edge_type = match args.get("edge_type").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: edge_type"),
    };
    let why = args.get("why").and_then(|v| v.as_str()).map(|s| s.to_string());

    let edge = EdgeRecord {
        id: Uuid::new_v4().to_string(),
        source_note_id: source_id,
        target_note_id: target_id,
        edge_type,
        why,
        from_source: crate::db::MANUAL_SOURCE_ID.to_string(),
        weight: 1.0,
        metadata: None,
        created_at: now_rfc3339(),
    };

    match db::insert_edge_if_not_exists(&ctx.db, &edge).await {
        Ok(inserted) => json_rpc_ok(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string(&json!({
                        "edge_id": if inserted { Value::String(edge.id.clone()) } else { Value::Null },
                        "inserted": inserted,
                    })).unwrap_or_default()
                }]
            }),
        ),
        Err(e) => json_rpc_err(id, -32000, &format!("Failed to insert edge: {e}")),
    }
}

// ---------------------------------------------------------------------------
// anansi_get_upload_url
// ---------------------------------------------------------------------------

async fn tool_get_upload_url(state: McpState, id: Value, args: Value) -> Json<Value> {
    let ctx = &state.ctx;

    // Derive base URL: use configured public_url, or fall back to localhost:{port}
    let base = ctx.config.server.public_url
        .as_deref()
        .map(|u| u.trim_end_matches('/').to_string())
        .unwrap_or_else(|| format!("http://localhost:{}", ctx.config.server.mcp_port));

    let inbox_url   = format!("{base}/upload/inbox");
    let atomize_url = format!("{base}/upload/atomize");

    let local_path = args.get("local_path").and_then(|v| v.as_str()).unwrap_or("{local_path}");

    let curl_inbox   = format!("curl -F 'file=@{local_path}' {inbox_url}");
    let curl_atomize = format!("curl -F 'file=@{local_path}' {atomize_url}");

    json_rpc_ok(
        id,
        json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&json!({
                    "inbox_url":   inbox_url,
                    "atomize_url": atomize_url,
                    "curl_inbox":  curl_inbox,
                    "curl_atomize": curl_atomize,
                    "note": "inbox = full pipeline (para-process → sb-atomize → DB). atomize = already-atomized content → DB directly."
                })).unwrap_or_default()
            }]
        }),
    )
}

// ---------------------------------------------------------------------------
// anansi_ingest_file
// ---------------------------------------------------------------------------

async fn tool_ingest_file(state: McpState, id: Value, args: Value) -> Json<Value> {
    let ctx = &state.ctx;

    if let Some(err) = check_skill_source(&id, &args) { return err; }

    if ctx.config.server.read_only {
        return json_rpc_err(id, -32000, "this anansi instance is read-only");
    }

    let content = match args.get("content").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None    => return json_rpc_err(id, -32602, "Missing required argument: content"),
    };

    // Use provided filename or generate capture-{YYYYMMDD-HHmmss}.md
    let filename = match args.get("filename").and_then(|v| v.as_str()) {
        Some(f) => {
            // Safety: reject path traversal
            if f.contains('/') || f.contains('\\') || f.contains("..") {
                return json_rpc_err(id, -32602, "filename must not contain path separators or '..'");
            }
            f.to_string()
        }
        None => {
            let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
            format!("capture-{ts}.md")
        }
    };

    let inbox_dir = &ctx.config.inbox.watch_dir;
    let dest = std::path::Path::new(inbox_dir).join(&filename);

    if let Err(e) = tokio::fs::create_dir_all(inbox_dir).await {
        return json_rpc_err(id, -32000, &format!("Cannot create inbox dir: {e}"));
    }
    if let Err(e) = tokio::fs::write(&dest, &content).await {
        return json_rpc_err(id, -32000, &format!("Failed to write to inbox: {e}"));
    }

    json_rpc_ok(
        id,
        json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&json!({
                    "status": "queued",
                    "queue_file": filename,
                    "queue_dir": inbox_dir,
                    "message": "Document queued for full pipeline processing (para-process → sb-atomize → ingest). Use anansi_filter or anansi_search to verify results after processing."
                })).unwrap_or_default()
            }]
        }),
    )
}

// ---------------------------------------------------------------------------
// anansi_ingest_atomized
// ---------------------------------------------------------------------------

async fn tool_ingest_atomized(state: McpState, id: Value, args: Value) -> Json<Value> {
    let ctx = &state.ctx;

    if let Some(err) = check_skill_source(&id, &args) { return err; }

    if ctx.config.server.read_only {
        return json_rpc_err(id, -32000, "this anansi instance is read-only");
    }

    // Resolve content_str: inline content preferred; else read from path
    let content_str = if let Some(c) = args.get("content").and_then(|v| v.as_str()) {
        c.to_string()
    } else if let Some(p) = args.get("path").and_then(|v| v.as_str()) {
        let raw = std::path::PathBuf::from(p);
        let canonical = match raw.canonicalize() {
            Ok(c) => c,
            Err(e) => return json_rpc_err(id, -32602, &format!("Cannot resolve path: {e}")),
        };
        if !canonical.starts_with(&*ctx.anansi_root) {
            return json_rpc_err(id, -32602, "path escapes anansi root");
        }
        match tokio::fs::read_to_string(&canonical).await {
            Ok(s) => s,
            Err(e) => return json_rpc_err(id, -32000, &format!("Failed to read file: {e}")),
        }
    } else {
        return json_rpc_err(id, -32602, "must provide either content or path");
    };

    // Write to queue dir — the queue watcher ingests it asynchronously
    let queue_dir = &ctx.config.inbox.queue_dir;
    let filename = format!("{}.md", Uuid::new_v4());
    let queue_path = std::path::Path::new(queue_dir).join(&filename);

    if let Err(e) = tokio::fs::create_dir_all(queue_dir).await {
        return json_rpc_err(id, -32000, &format!("Cannot create queue dir: {e}"));
    }
    if let Err(e) = tokio::fs::write(&queue_path, &content_str).await {
        return json_rpc_err(id, -32000, &format!("Failed to write to queue: {e}"));
    }

    json_rpc_ok(
        id,
        json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&json!({
                    "status": "queued",
                    "queue_file": filename,
                    "message": "Atomized content queued for ingestion. Use anansi_filter or anansi_search to verify results in a few seconds."
                })).unwrap_or_default()
            }]
        }),
    )
}

// ---------------------------------------------------------------------------
// anansi_capture
// ---------------------------------------------------------------------------

async fn tool_capture(state: McpState, id: Value, args: Value) -> Json<Value> {
    let ctx = &state.ctx;

    if let Some(err) = check_skill_source(&id, &args) { return err; }

    if ctx.config.server.read_only {
        return json_rpc_err(id, -32000, "this anansi instance is read-only");
    }

    let entity_type = match args.get("entity_type").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: entity_type"),
    };
    let name = match args.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: name"),
    };
    let lede = match args.get("lede").and_then(|v| v.as_str()) {
        Some(l) => l.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: lede"),
    };
    let why = args.get("why").and_then(|v| v.as_str()).map(|s| s.to_string());
    let content = args.get("content").and_then(|v| v.as_str()).map(|s| s.to_string());

    let match_key = db::match_key(&name, &entity_type);

    // Check if note already exists
    let existing = db::find_note_by_match_key(&ctx.db, &match_key).await;
    let existed = matches!(&existing, Ok(Some(_)));
    let note_id = existing
        .ok()
        .flatten()
        .map(|n| n.id)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let rec = db::NoteRecord {
        id: note_id.clone(),
        entity_type,
        name: name.clone(),
        match_key: match_key.clone(),
        lede: Some(lede),
        why,
        content,
        has_conflicts: 0,
        conflicts_updated_at: None,
        merge_category: "entity".to_string(),
        created_from: db::MANUAL_SOURCE_ID.to_string(),
        source_count: 1,
        created_at: db::now_rfc3339(),
        updated_at: db::now_rfc3339(),
    };

    // Capture-specific upsert: new values WIN (not COALESCE).
    // Content is appended so facts accumulate across captures.
    let result = sqlx::query(
        "INSERT INTO notes \
         (id, entity_type, name, match_key, lede, why, content, \
          has_conflicts, conflicts_updated_at, merge_category, created_from, \
          source_count, created_at, updated_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,0,NULL,'entity',$8,1,$9,$10) \
         ON CONFLICT(match_key) DO UPDATE SET \
           lede = excluded.lede, \
           why = CASE WHEN excluded.why IS NOT NULL THEN excluded.why ELSE notes.why END, \
           content = CASE \
             WHEN notes.content IS NULL THEN excluded.content \
             WHEN excluded.content IS NULL THEN notes.content \
             ELSE notes.content || chr(10) || excluded.content \
           END, \
           source_count = notes.source_count + 1, \
           updated_at = excluded.updated_at",
    )
    .bind(&rec.id)
    .bind(&rec.entity_type)
    .bind(&rec.name)
    .bind(&rec.match_key)
    .bind(&rec.lede)
    .bind(&rec.why)
    .bind(&rec.content)
    .bind(db::MANUAL_SOURCE_ID)
    .bind(&rec.created_at)
    .bind(&rec.updated_at)
    .execute(&ctx.db)
    .await;

    if let Err(e) = result {
        return json_rpc_err(id, -32000, &format!("Capture failed: {e:#}"));
    }

    let status = if existed { "updated" } else { "created" };

    json_rpc_ok(
        id,
        json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&json!({
                    "status": status,
                    "note_id": note_id,
                    "match_key": match_key,
                    "name": name,
                })).unwrap_or_default()
            }]
        }),
    )
}
// ---------------------------------------------------------------------------
// anansi_delete_note
// ---------------------------------------------------------------------------

async fn tool_delete_note(state: McpState, id: Value, args: Value) -> Json<Value> {
    let ctx = &state.ctx;

    if let Some(err) = check_skill_source(&id, &args) { return err; }

    if ctx.config.server.read_only {
        return json_rpc_err(id, -32000, "this anansi instance is read-only");
    }

    let note_id = match args.get("note_id").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: note_id"),
    };

    // Verify the note exists first
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM notes WHERE id = $1)")
        .bind(&note_id)
        .fetch_one(&ctx.db)
        .await
        .unwrap_or(false);

    if !exists {
        return json_rpc_err(id, -32000, &format!("Note not found: {note_id}"));
    }

    // Delete edges connected to this note (both directions) — no FK cascade on edges
    let r1 = sqlx::query("DELETE FROM edges WHERE source_note_id = $1")
        .bind(&note_id).execute(&ctx.db).await;
    let r2 = sqlx::query("DELETE FROM edges WHERE target_note_id = $1")
        .bind(&note_id).execute(&ctx.db).await;
    let edges_deleted = r1.map(|r| r.rows_affected()).unwrap_or(0)
        + r2.map(|r| r.rows_affected()).unwrap_or(0);

    // Delete source contributions — no FK cascade
    let contribs_deleted =
        sqlx::query("DELETE FROM source_contributions WHERE note_id = $1")
            .bind(&note_id)
            .execute(&ctx.db)
            .await
            .map(|r| r.rows_affected())
            .unwrap_or(0);

    // Delete the note itself — embeddings cascade automatically
    let deleted = sqlx::query("DELETE FROM notes WHERE id = $1")
        .bind(&note_id)
        .execute(&ctx.db)
        .await
        .map(|r| r.rows_affected())
        .unwrap_or(0);

    if deleted == 0 {
        return json_rpc_err(id, -32000, &format!("Failed to delete note: {note_id}"));
    }

    json_rpc_ok(id, json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json!({
                "status": "deleted",
                "note_id": note_id,
                "edges_deleted": edges_deleted,
                "contributions_deleted": contribs_deleted,
            })).unwrap_or_default()
        }]
    }))
}

// ---------------------------------------------------------------------------
// anansi_archive_note
// ---------------------------------------------------------------------------

async fn tool_archive_note(state: McpState, id: Value, args: Value) -> Json<Value> {
    let ctx = &state.ctx;

    if let Some(err) = check_skill_source(&id, &args) { return err; }

    if ctx.config.server.read_only {
        return json_rpc_err(id, -32000, "this anansi instance is read-only");
    }

    let note_id = match args.get("note_id").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: note_id"),
    };
    let restore = args.get("restore").and_then(|v| v.as_bool()).unwrap_or(false);

    // Fetch current entity_type
    let current_type: Option<String> =
        sqlx::query_scalar("SELECT entity_type FROM notes WHERE id = $1")
            .bind(&note_id)
            .fetch_optional(&ctx.db)
            .await
            .unwrap_or(None);

    let current_type = match current_type {
        Some(t) => t,
        None => return json_rpc_err(id, -32000, &format!("Note not found: {note_id}")),
    };

    let new_type = if restore {
        // Strip leading "archive-" prefix(es)
        current_type.trim_start_matches("archive-").to_string()
    } else {
        if current_type.starts_with("archive-") {
            return json_rpc_err(id, -32000, "Note is already archived. Pass restore:true to unarchive.");
        }
        format!("archive-{current_type}")
    };

    sqlx::query("UPDATE notes SET entity_type = $1, updated_at = NOW() WHERE id = $2")
        .bind(&new_type)
        .bind(&note_id)
        .execute(&ctx.db)
        .await
        .map_err(|e| e.to_string())
        .ok();

    let action = if restore { "restored" } else { "archived" };
    json_rpc_ok(id, json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string(&json!({
                "status": action,
                "note_id": note_id,
                "entity_type": new_type,
            })).unwrap_or_default()
        }]
    }))
}

// ---------------------------------------------------------------------------
// anansi_purge
// ---------------------------------------------------------------------------

async fn tool_purge(state: McpState, id: Value, args: Value) -> Json<Value> {
    let ctx = &state.ctx;

    if let Some(err) = check_skill_source(&id, &args) { return err; }

    if ctx.config.server.read_only {
        return json_rpc_err(id, -32000, "this anansi instance is read-only");
    }

    let source_id = match args.get("source_id").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: source_id"),
    };

    // Find all note IDs associated with this source:
    // 1. Notes where created_from = source_id (direct creation)
    // 2. Notes referenced in source_contributions for this source (merged notes)
    let note_ids: Vec<String> = {
        let direct = sqlx::query_scalar::<_, String>(
            "SELECT id FROM notes WHERE created_from = $1"
        )
        .bind(&source_id)
        .fetch_all(&ctx.db)
        .await
        .unwrap_or_default();

        let contributed = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT note_id FROM source_contributions WHERE source_id = $1"
        )
        .bind(&source_id)
        .fetch_all(&ctx.db)
        .await
        .unwrap_or_default();

        // Union the two sets
        let mut ids: std::collections::HashSet<String> = direct.into_iter().collect();
        ids.extend(contributed);
        ids.into_iter().collect()
    };

    // Delete edges connected to any of these notes (must be before notes deletion)
    let mut edges_deleted: u64 = 0;
    for nid in &note_ids {
        let r1 = sqlx::query("DELETE FROM edges WHERE source_note_id = $1")
            .bind(nid).execute(&ctx.db).await;
        let r2 = sqlx::query("DELETE FROM edges WHERE target_note_id = $1")
            .bind(nid).execute(&ctx.db).await;
        edges_deleted += r1.map(|r| r.rows_affected()).unwrap_or(0);
        edges_deleted += r2.map(|r| r.rows_affected()).unwrap_or(0);
    }
    // Also delete any edges attributed to this source via from_source FK
    let r3 = sqlx::query("DELETE FROM edges WHERE from_source = $1")
        .bind(&source_id).execute(&ctx.db).await;
    edges_deleted += r3.map(|r| r.rows_affected()).unwrap_or(0);

    // Delete source contributions for this source (must be before notes deletion)
    let contribs_deleted = sqlx::query("DELETE FROM source_contributions WHERE source_id = $1")
        .bind(&source_id)
        .execute(&ctx.db)
        .await
        .map(|r| r.rows_affected())
        .unwrap_or(0);

    // Delete notes. ON DELETE CASCADE on note_embeddings FK handles embedding cleanup.
    // Deletion order: edges → contributions → notes (satisfies all FK constraints).
    let mut notes_deleted: u64 = 0;
    for nid in &note_ids {
        if let Ok(r) = sqlx::query("DELETE FROM notes WHERE id = $1")
            .bind(nid)
            .execute(&ctx.db)
            .await
        {
            notes_deleted += r.rows_affected();
        }
    }

    // Always delete the source record, even if 0 notes were found
    let _ = sqlx::query("DELETE FROM sources WHERE id = $1")
        .bind(&source_id)
        .execute(&ctx.db)
        .await;

    json_rpc_ok(
        id,
        json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&json!({
                    "status": "purged",
                    "source_id": source_id,
                    "notes_deleted": notes_deleted,
                    "edges_deleted": edges_deleted,
                    "contributions_deleted": contribs_deleted,
                })).unwrap_or_default()
            }]
        }),
    )
}


// ---------------------------------------------------------------------------
// GET /health
// ---------------------------------------------------------------------------

async fn handle_health(State(state): State<McpState>) -> impl IntoResponse {
    let ctx = &state.ctx;

    // Check LLM reachability (optional — may not be configured)
    let llm_status = if let Some(ref llm) = ctx.llm {
        if llm.ping().await.is_ok() { "reachable" } else { "unreachable" }
    } else {
        "not_configured"
    };

    // Check DB with SELECT 1
    let db_ok = sqlx::query("SELECT 1")
        .execute(&ctx.db)
        .await
        .is_ok();
    let db_status = if db_ok { "ok" } else { "error" };

    // Server is healthy as long as DB works — LLM is optional
    let status = if db_ok { "ok" } else { "degraded" };

    let body = json!({
        "status": status,
        "llm": llm_status,
        "db": db_status,
        "version": "0.1.0",
    });

    let status_code = if status == "ok" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(body))
}

// ---------------------------------------------------------------------------
// File upload endpoints  POST /upload/inbox  and  POST /upload/atomize
// ---------------------------------------------------------------------------
//
// curl -F "file=@myfile.md" http://localhost:3738/upload/inbox
// curl -F "file=@atomized.md" http://localhost:3738/upload/atomize
//
// Uses ctx.config.inbox.watch_dir / queue_dir — same paths as the MCP tools
// and the queue/inbox watchers — so no path adjustment is needed for deploy.

async fn handle_upload_inbox(
    State(state): State<McpState>,
    multipart: Multipart,
) -> impl IntoResponse {
    handle_upload(state, multipart, UploadDest::Inbox).await
}

async fn handle_upload_atomize(
    State(state): State<McpState>,
    multipart: Multipart,
) -> impl IntoResponse {
    handle_upload(state, multipart, UploadDest::Atomize).await
}

enum UploadDest { Inbox, Atomize }

async fn handle_upload(
    state: McpState,
    mut multipart: Multipart,
    dest: UploadDest,
) -> impl IntoResponse {
    let ctx = &state.ctx;

    if ctx.config.server.read_only {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "this anansi instance is read-only" })),
        );
    }

    let dir = match dest {
        UploadDest::Inbox   => &ctx.config.inbox.watch_dir,
        UploadDest::Atomize => &ctx.config.inbox.queue_dir,
    };

    // Extract the `file` field from the multipart body
    while let Ok(Some(field)) = multipart.next_field().await {
        // Accept field named "file" or the first unnamed field
        let field_name = field.name().unwrap_or("").to_string();
        if !field_name.is_empty() && field_name != "file" {
            continue;
        }

        // Determine filename: from Content-Disposition, or generate timestamp
        let filename = field
            .file_name()
            .filter(|n| !n.is_empty() && !n.contains('/') && !n.contains(".."))
            .map(|n| n.to_string())
            .unwrap_or_else(|| {
                let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
                format!("upload-{ts}.md")
            });

        let data = match field.bytes().await {
            Ok(b) => b,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("failed to read upload: {e}") })),
                );
            }
        };

        if let Err(e) = tokio::fs::create_dir_all(dir).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("cannot create dir: {e}") })),
            );
        }

        let dest_path = std::path::Path::new(dir).join(&filename);
        if let Err(e) = tokio::fs::write(&dest_path, &data).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("write failed: {e}") })),
            );
        }

        return (
            StatusCode::OK,
            Json(json!({
                "status": "queued",
                "filename": filename,
                "queue_dir": dir,
                "bytes": data.len(),
            })),
        );
    }

    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": "no file field found in multipart body" })),
    )
}

// ---------------------------------------------------------------------------
// anansi_export_context
// ---------------------------------------------------------------------------

async fn tool_export_context(state: McpState, id: Value, args: Value) -> Json<Value> {
    let note_id = match args.get("id").and_then(|v| v.as_str()) {
        Some(i) => i.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: id"),
    };
    let depth = args
        .get("depth")
        .and_then(|v| v.as_u64())
        .unwrap_or(2)
        .clamp(1, 5) as usize;

    match export::export_context(&state.ctx.db, &note_id, depth).await {
        Ok(doc) => json_rpc_ok(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": doc
                }]
            }),
        ),
        Err(e) => json_rpc_err(id, -32000, &format!("Export failed: {e:#}")),
    }
}

// ---------------------------------------------------------------------------
// anansi_export_vault
// ---------------------------------------------------------------------------

async fn tool_export_vault(state: McpState, id: Value, args: Value) -> Json<Value> {
    let note_id = match args.get("id").and_then(|v| v.as_str()) {
        Some(i) => i.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: id"),
    };
    let depth = args
        .get("depth")
        .and_then(|v| v.as_u64())
        .unwrap_or(2)
        .clamp(1, 5) as usize;

    let exports_dir = state.ctx.anansi_root.join("exports");
    match export::export_vault(&state.ctx.db, &note_id, depth, &exports_dir).await {
        Ok(zip_name) => {
            let url = state.ctx.config.server.public_url
                .as_deref()
                .map(|base| format!("{base}/exports/{zip_name}"))
                .unwrap_or_else(|| format!("/exports/{zip_name}"));
            json_rpc_ok(
                id,
                json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&json!({
                            "status": "ready",
                            "filename": zip_name,
                            "download_url": url,
                        })).unwrap_or_default()
                    }]
                }),
            )
        }
        Err(e) => json_rpc_err(id, -32000, &format!("Vault export failed: {e:#}")),
    }
}

// ---------------------------------------------------------------------------
// anansi_embed
// ---------------------------------------------------------------------------

async fn tool_embed(state: McpState, id: Value, args: Value) -> Json<Value> {
    let ctx = &state.ctx;

    if ctx.config.server.read_only {
        return json_rpc_err(id, -32000, "this anansi instance is read-only");
    }

    let api_key = match ctx.config.llm.gemini.as_ref().and_then(|g| g.api_key.as_deref()) {
        Some(k) => k.to_string(),
        None => return json_rpc_err(id, -32000, "No Gemini API key configured (set ANANSI_GEMINI_API_KEY)"),
    };

    let model = "text-embedding-004";

    if let Some(note_id) = args.get("note_id").and_then(|v| v.as_str()) {
        // Single-note embed
        let note = match db::get_note(&ctx.db, note_id).await {
            Ok(Some(n)) => n,
            Ok(None) => return json_rpc_err(id, -32602, &format!("Note not found: {note_id}")),
            Err(e) => return json_rpc_err(id, -32000, &format!("DB error: {e:#}")),
        };

        let text = format!(
            "{} {} {} {}",
            note.name,
            note.lede.as_deref().unwrap_or(""),
            note.why.as_deref().unwrap_or(""),
            note.content.as_deref().unwrap_or("")
        );

        let vec = match embed::gemini_embed(&api_key, &text).await {
            Ok(v) => v,
            Err(e) => return json_rpc_err(id, -32000, &format!("Embed failed: {e:#}")),
        };

        let vector = pgvector::Vector::from(vec);
        let now = now_rfc3339();
        let result = sqlx::query(
            "INSERT INTO note_embeddings (note_id, model, embedding, embedded_at) \
             VALUES ($1,$2,$3,$4) \
             ON CONFLICT (note_id, model) DO UPDATE SET embedding = EXCLUDED.embedding, embedded_at = EXCLUDED.embedded_at"
        )
        .bind(note_id)
        .bind(model)
        .bind(vector)
        .bind(&now)
        .execute(&ctx.db)
        .await;

        match result {
            Ok(_) => json_rpc_ok(id, json!({ "content": [{ "type": "text", "text": format!("1 note embedded") }] })),
            Err(e) => json_rpc_err(id, -32000, &format!("Insert failed: {e:#}")),
        }
    } else if args.get("batch").and_then(|v| v.as_bool()).unwrap_or(false) {
        // Batch embed: up to 100 notes without an embedding for this model
        let note_ids: Vec<String> = match sqlx::query_scalar::<_, String>(
            "SELECT id FROM notes WHERE id NOT IN \
             (SELECT note_id FROM note_embeddings WHERE model = $1) \
             LIMIT 100"
        )
        .bind(model)
        .fetch_all(&ctx.db)
        .await {
            Ok(ids) => ids,
            Err(e) => return json_rpc_err(id, -32000, &format!("DB query failed: {e:#}")),
        };

        let mut embedded = 0u32;
        let mut errors = 0u32;

        for note_id in &note_ids {
            let note = match db::get_note(&ctx.db, note_id).await {
                Ok(Some(n)) => n,
                _ => { errors += 1; continue; }
            };

            let text = format!(
                "{} {} {} {}",
                note.name,
                note.lede.as_deref().unwrap_or(""),
                note.why.as_deref().unwrap_or(""),
                note.content.as_deref().unwrap_or("")
            );

            let vec = match embed::gemini_embed(&api_key, &text).await {
                Ok(v) => v,
                Err(_) => { errors += 1; continue; }
            };

            let vector = pgvector::Vector::from(vec);
            let now = now_rfc3339();
            let _ = sqlx::query(
                "INSERT INTO note_embeddings (note_id, model, embedding, embedded_at) \
                 VALUES ($1,$2,$3,$4) \
                 ON CONFLICT (note_id, model) DO UPDATE SET embedding = EXCLUDED.embedding, embedded_at = EXCLUDED.embedded_at"
            )
            .bind(note_id)
            .bind(model)
            .bind(vector)
            .bind(&now)
            .execute(&ctx.db)
            .await;

            embedded += 1;
        }

        json_rpc_ok(id, json!({
            "content": [{
                "type": "text",
                "text": format!("{embedded} notes embedded, {errors} errors")
            }]
        }))
    } else {
        json_rpc_err(id, -32602, "Provide either note_id or batch:true")
    }
}

// ---------------------------------------------------------------------------
// anansi_search_semantic
// ---------------------------------------------------------------------------

async fn tool_search_semantic(state: McpState, id: Value, args: Value) -> Json<Value> {
    let ctx = &state.ctx;

    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q.to_string(),
        None => return json_rpc_err(id, -32602, "Missing required argument: query"),
    };

    let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(10);
    let model = args.get("model").and_then(|v| v.as_str()).unwrap_or("text-embedding-004").to_string();

    let api_key = match ctx.config.llm.gemini.as_ref().and_then(|g| g.api_key.as_deref()) {
        Some(k) => k.to_string(),
        None => return json_rpc_err(id, -32000, "No Gemini API key configured (set ANANSI_GEMINI_API_KEY)"),
    };

    let vec = match embed::gemini_embed(&api_key, &query).await {
        Ok(v) => v,
        Err(e) => return json_rpc_err(id, -32000, &format!("Embed query failed: {e:#}")),
    };

    let vector = pgvector::Vector::from(vec);

    let rows = match sqlx::query(
        "SELECT n.id, n.name, n.entity_type, n.lede, n.match_key, \
         1-(ne.embedding<=>$1) AS similarity \
         FROM note_embeddings ne \
         JOIN notes n ON n.id = ne.note_id \
         WHERE ne.model = $2 \
         ORDER BY ne.embedding<=>$1 \
         LIMIT $3"
    )
    .bind(&vector)
    .bind(&model)
    .bind(limit)
    .fetch_all(&ctx.db)
    .await {
        Ok(r) => r,
        Err(e) => return json_rpc_err(id, -32000, &format!("Semantic search failed: {e:#}")),
    };

    use sqlx::Row;
    let results: Vec<serde_json::Value> = rows.iter().map(|r| {
        let similarity: f64 = r.try_get("similarity").unwrap_or(0.0);
        json!({
            "id": r.get::<String, _>("id"),
            "name": r.get::<String, _>("name"),
            "entity_type": r.get::<String, _>("entity_type"),
            "lede": r.try_get::<Option<String>, _>("lede").unwrap_or(None),
            "match_key": r.get::<String, _>("match_key"),
            "similarity": (similarity * 1000.0).round() / 1000.0,
        })
    }).collect();

    let text = serde_json::to_string_pretty(&results).unwrap_or_default();

    json_rpc_ok(id, json!({
        "content": [{
            "type": "text",
            "text": text
        }]
    }))
}

// ---------------------------------------------------------------------------
// GET /exports/:filename  — serve generated zip files
// ---------------------------------------------------------------------------

async fn handle_export_download(
    State(state): State<McpState>,
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> impl IntoResponse {
    // Security: reject any path traversal
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return (
            StatusCode::BAD_REQUEST,
            [("content-type", "text/plain")],
            axum::body::Bytes::from("Invalid filename"),
        );
    }

    let path = state.ctx.anansi_root.join("exports").join(&filename);
    match tokio::fs::read(&path).await {
        Ok(bytes) => (
            StatusCode::OK,
            [(
                "content-type",
                "application/zip",
            )],
            axum::body::Bytes::from(bytes),
        ),
        Err(_) => (
            StatusCode::NOT_FOUND,
            [("content-type", "text/plain")],
            axum::body::Bytes::from("Export not found"),
        ),
    }
}
