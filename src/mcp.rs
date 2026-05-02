use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::atomized_ingest;
use crate::db::{self, EdgeRecord, now_rfc3339};
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
                    "description": "Full-text search across note name, lede, why, and content using FTS5. Supports FTS5 query syntax (e.g. phrase in quotes, AND/OR, prefix*). Use for keyword and concept lookups.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "FTS5 search query. Plain terms, quoted phrases, or operators like AND/OR/NOT." },
                            "limit": { "type": "integer", "description": "Max results (default 20)." }
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
                    "description": "Filter notes by entity_type and/or date range. All parameters optional. Use to list all notes of a type, or all notes updated in a time window.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "entity_type": { "type": "string", "description": "Filter by type, e.g. person, organization, project, event, topic, note." },
                            "after": { "type": "string", "description": "ISO 8601 datetime — return notes updated at or after this timestamp." },
                            "before": { "type": "string", "description": "ISO 8601 datetime — return notes updated at or before this timestamp." },
                            "limit": { "type": "integer", "description": "Max results (default 50)." }
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
                    "description": "Delete all notes, edges, and contributions from a specific import by source_id. Removes the source record too. Use anansi_search or Datasette to find source_ids.",
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
        "anansi_ingest_atomized" => tool_ingest_atomized(state, id, args).await,
        "anansi_capture" => tool_capture(state, id, args).await,
        "anansi_purge" => tool_purge(state, id, args).await,
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

    match db::search_notes(&state.ctx.db, &query, limit).await {
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

    match db::filter_notes(&state.ctx.db, entity_type, after, before, limit).await {
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

    let para_toc = args.get("para_toc").and_then(|v| v.as_str());
    let source_path = args.get("source_path").and_then(|v| v.as_str());

    match atomized_ingest::ingest_atomized(&ctx.db, &content_str, para_toc, source_path).await {
        Ok(report) if report.status == "already_ingested" => json_rpc_ok(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string(&json!({
                        "status": "already_ingested",
                        "source_id": report.source_id,
                    })).unwrap_or_default()
                }]
            }),
        ),
        Ok(report) => json_rpc_ok(
            id,
            json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string(&report).unwrap_or_default()
                }]
            }),
        ),
        Err(e) => json_rpc_err(id, -32000, &format!("Ingest failed: {e:#}")),
    }
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
         VALUES (?,?,?,?,?,?,?,0,NULL,'entity',?,1,?,?) \
         ON CONFLICT(match_key) DO UPDATE SET \
           lede = excluded.lede, \
           why = CASE WHEN excluded.why IS NOT NULL THEN excluded.why ELSE why END, \
           content = CASE \
             WHEN content IS NULL THEN excluded.content \
             WHEN excluded.content IS NULL THEN content \
             ELSE content || char(10) || excluded.content \
           END, \
           source_count = source_count + 1, \
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

    // Find all note IDs created from this source
    let note_ids: Vec<String> = match sqlx::query_scalar::<_, String>(
        "SELECT id FROM notes WHERE created_from = ?"
    )
    .bind(&source_id)
    .fetch_all(&ctx.db)
    .await {
        Ok(ids) => ids,
        Err(e) => return json_rpc_err(id, -32000, &format!("Query failed: {e:#}")),
    };

    if note_ids.is_empty() {
        return json_rpc_err(id, -32000, &format!("No notes found for source_id {source_id}"));
    }

    // Delete edges where either side is a note from this source
    let mut edges_deleted: u64 = 0;
    for nid in &note_ids {
        let r1 = sqlx::query("DELETE FROM edges WHERE source_note_id = ?")
            .bind(nid).execute(&ctx.db).await;
        let r2 = sqlx::query("DELETE FROM edges WHERE target_note_id = ?")
            .bind(nid).execute(&ctx.db).await;
        edges_deleted += r1.map(|r| r.rows_affected()).unwrap_or(0);
        edges_deleted += r2.map(|r| r.rows_affected()).unwrap_or(0);
    }

    // Delete source contributions
    let contribs = sqlx::query("DELETE FROM source_contributions WHERE source_id = ?")
        .bind(&source_id).execute(&ctx.db).await;
    let contribs_deleted = contribs.map(|r| r.rows_affected()).unwrap_or(0);

    // Delete the notes
    let notes = sqlx::query("DELETE FROM notes WHERE created_from = ?")
        .bind(&source_id).execute(&ctx.db).await;
    let notes_deleted = notes.map(|r| r.rows_affected()).unwrap_or(0);

    // Delete the source record
    let _ = sqlx::query("DELETE FROM sources WHERE id = ?")
        .bind(&source_id).execute(&ctx.db).await;

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
