# Working Plan: `attach_to_outline` Option for `anansi_capture`

## Objective

Add an optional `attach_to_outline` parameter to the `anansi_capture` MCP tool (`src/mcp.rs`) so that a single call can:

1. Create or update a fact note (existing behavior).
2. Insert a `part_of` edge from the new note to a specified outline note, using the manual-edge sentinel source `db::MANUAL_SOURCE_ID`.
3. Append the new note's `match_key` to the outline's `content` field.

This replaces the current manual 3-call pattern (capture → relate → update). When the parameter is omitted, behavior is byte-identical to today.

---

## Constraints

- **Language:** Rust (edition per `Cargo.toml`).
- **Framework:** `axum`-based MCP JSON-RPC server; `sqlx` (Postgres); `tokio` async runtime.
- **No new dependencies** — all required crates (`uuid`, `async_trait`, `tempfile`, `serde_json`, `sqlx`) are already in the dependency tree.
- **No DB migrations** — the `edges` and `notes` tables already support the required operations. The unique constraint on `edges (source_note_id, target_note_id, edge_type, from_source)` makes `insert_edge_if_not_exists` idempotent.
- **No changes to `db.rs`** — all needed DB helpers are already `pub`:
  - `db::find_note_by_match_key(pool, &str) -> Result<Option<NoteRecord>>` (`db.rs:208`)
  - `db::get_note(pool, &str) -> Result<Option<NoteRecord>>` (`db.rs:200`)
  - `db::insert_edge_if_not_exists(pool, &EdgeRecord) -> Result<bool>` (`db.rs:394`, unique on `(source_note_id, target_note_id, edge_type, from_source)`)
  - `db::edges_for_note(pool, &str) -> Result<Vec<EdgeRecord>>` (`db.rs:415`)
  - `db::insert_note(pool, &NoteRecord) -> Result<()>` (`db.rs:167`, for test seed)
  - `db::MANUAL_SOURCE_ID = "00000000-0000-0000-0000-000000000000"` (`db.rs:10`)
  - `db::now_rfc3339()` and `db::match_key(name, entity_type)` (already in scope via `use crate::db::{self, now_rfc3339, EdgeRecord};` at `mcp.rs:17`)
- **`Uuid` already imported** at `mcp.rs:15` (`use uuid::Uuid;`).
- **Edge direction** must match the canonical `part_of` convention confirmed at `atomized_ingest.rs:179-194`: `source_note_id = block/leaf note`, `target_note_id = outline`. This ensures `anansi_edges <outline-id> part_of` discovers the captured note as a BFS neighbor (the BFS at `tool_edges` (`mcp.rs:916`) traverses both `source_note_id` and `target_note_id` via `edges_for_note`).
- **`check_skill_source`** (`mcp.rs:81`) rejects calls unless `args.source == "skill"`. Tests must pass `"source": "skill"`. The `read_only` guard at `mcp.rs:~1450` runs before any new logic — attach is correctly blocked in read-only mode.
- **`tool_capture` and `tool_edges` are currently private** (`async fn`, not `pub`). Integration tests in `tests/` need them accessible. The module `mcp` is `pub mod mcp` in `lib.rs:11`, so marking the functions `pub` exposes them to integration tests. Alternatively, tests can be inline `#[cfg(test)] mod tests` inside `mcp.rs`.

---

## File Structure

### Modify

| File | Change |
|------|--------|
| `src/mcp.rs` | (1) Add `attach_to_outline` property to the `anansi_capture` `inputSchema.properties` (~line 341, after `source`). (2) Make `tool_capture` `pub` for test access (line 1444: `async fn` → `pub async fn`). (3) Make `tool_edges` `pub` for test access (line 916). (4) Insert attach logic after the wiki-materialize block and before `let status = ...` (~line 1557). (5) Add `"attached_to"` field to the response JSON. |

### Create

| File | Purpose |
|------|---------|
| `tests/mcp_capture.rs` | New integration test file with 3 tests, gated on `DATABASE_URL` env var (skip cleanly when unset). Reuses the `make_context`-style pattern from `tests/pipeline_integration.rs`. |

### Unchanged

| File | Rationale |
|------|-----------|
| `src/db.rs` | All required helpers already exist and are `pub`. |
| `Cargo.toml` | No new dependencies. |
| `src/atomized_ingest.rs` | Reference-only; the `part_of` edge pattern here is the canonical convention we mirror, not a file to change. |
| `tests/pipeline_integration.rs` | Do not modify existing tests. New tests go in a separate file. |

---

## Implementation Notes

### 1. Input Schema (`src/mcp.rs` ~line 341)

Add to the `anansi_capture` `inputSchema.properties`, after the `source` property:

```json
"attach_to_outline": {
    "type": "string",
    "description": "Optional. match_key or id of an outline note to attach this note to. Creates a part_of edge (this note → outline) and appends this note's match_key to the outline's content field. Replaces the manual 3-call capture+relate+update pattern. No-op when omitted."
}
```

Do **not** add to `required` — it must remain optional.

### 2. Attach Logic (`src/mcp.rs`, after wiki-materialize, before `let status = ...`)

Insert the following block. Key design decisions are called out in comments:

```rust
// Optional: attach the captured note to an outline via a part_of edge and
// append the note's match_key to the outline content. Replaces the manual
// 3-call capture + relate + update pattern. No-op when omitted.
let attached_to: Option<String> =
    if let Some(target) = args.get("attach_to_outline").and_then(|v| v.as_str()) {
        // Resolve the outline by match_key (contains ':') or by id (UUID).
        let outline = if target.contains(':') {
            db::find_note_by_match_key(&ctx.db, target).await.ok().flatten()
        } else {
            db::get_note(&ctx.db, target).await.ok().flatten()
        };
        match outline {
            Some(outline_note) => {
                // Guard: skip self-attach (prevents self-loop part_of edge).
                if outline_note.id == note_id {
                    None
                } else {
                    // part_of edge: captured note -> outline, manual sentinel source.
                    let edge = EdgeRecord {
                        id: Uuid::new_v4().to_string(),
                        source_note_id: note_id.clone(),
                        target_note_id: outline_note.id.clone(),
                        edge_type: "part_of".to_string(),
                        why: Some(format!(
                            "Captured note {} attached to outline {}",
                            match_key, outline_note.match_key
                        )),
                        from_source: db::MANUAL_SOURCE_ID.to_string(),
                        weight: 1.0,
                        metadata: None,
                        created_at: now_rfc3339(),
                    };
                    if let Err(e) = db::insert_edge_if_not_exists(&ctx.db, &edge).await {
                        eprintln!("[capture] attach part_of edge failed: {e}");
                    }

                    // Append the captured note's match_key to the outline content.
                    let append = format!("\n{match_key}");
                    let upd = sqlx::query(
                        "UPDATE notes SET \
                            content = CASE WHEN content IS NULL THEN $2 \
                                         ELSE content || $2 END, \
                            updated_at = $3 \
                         WHERE id = $1",
                    )
                    .bind(&outline_note.id)
                    .bind(&append)
                    .bind(now_rfc3339())
                    .execute(&ctx.db)
                    .await;
                    if let Err(e) = upd {
                        eprintln!("[capture] outline content append failed: {e}");
                    }

                    // Re-project the outline so the appended line shows in the wiki (non-fatal).
                    let wiki = WikiStore::from_config(&ctx.config);
                    if let Err(e) = wiki
                        .materialize(&ctx.db, &[outline_note.id.clone()], "capture-attach")
                        .await
                    {
                        eprintln!("[wiki] capture-attach outline materialize failed: {e}");
                    }
                    Some(outline_note.match_key.clone())
                }
            }
            None => {
                return json_rpc_err(
                    id,
                    -32000,
                    &format!("attach_to_outline: outline not found for '{target}'"),
                );
            }
        }
    } else {
        None
    };
```

### 3. Response (`src/mcp.rs`, inside the `json!({...})` response block)

Add `"attached_to": attached_to` to the response. When the param is omitted, `attached_to` is `None` → serializes to `null`. Existing callers ignore unknown response keys.

### Design Decisions (Consensus and Divergence)

| Decision | Consensus | Divergence | Resolution |
|----------|-----------|------------|------------|
| **Outline resolution heuristic** | All 4 agents agree: `target.contains(':')` → match_key, else → UUID. | — | **Accept.** The acceptance example `outline:anansi-offload-index` contains `:`, matching match_key format confirmed at `atomized_ingest.rs:107-114`. |
| **Edge direction** | All 4 agents agree: `source_note_id = captured note`, `target_note_id = outline`, `edge_type = "part_of"`. | — | **Accept.** Confirmed at `atomized_ingest.rs:179-194`. This is critical for `anansi_edges` BFS discovery — `edges_for_note` (`db.rs:415`) queries `WHERE source_note_id = $1 OR target_note_id = $1`, so the outline node will find the edge and the BFS will discover the captured note as a neighbor. |
| **`from_source`** | All 4 agents agree: `db::MANUAL_SOURCE_ID`. | — | **Accept.** Per the request, and matches `tool_relate` usage. |
| **Error handling: outline not found** | [GLM] and [DEEPSEEK] favor **fatal error** (`json_rpc_err`). [QWEN] and [NEMOTRON] favor **non-fatal** (log + continue). | **Reject [QWEN] and [NEMOTRON] non-fatal approach.** The contract is "one call does all three." If the outline doesn't exist, the caller needs to know the attach failed. The note is already persisted (upsert completed), so the capture itself succeeds — but the attach contract is violated. Returning `-32000` with a clear message is more informative. **However**, [GLM]'s approach of returning the error *after* the note is already saved is the correct semantics: the note persists, but the RPC signals the attach failure. | **Accept [GLM]/[DEEPSEEK] fatal-on-not-found.** |
| **Error handling: edge/content write failure** | [GLM] and [QWEN] and [NEMOTRON]: non-fatal (`eprintln!` + continue). [DEEPSEEK]: fatal. | The edge insert is idempotent (`ON CONFLICT DO NOTHING`), so failure is unlikely and re-running is safe. Content append failure is more serious but still non-fatal — the note is saved, the edge may be saved, only the content append failed. **Accept non-fatal for write failures** (edge insert + content append), matching the existing `wiki.materialize` error pattern in `tool_capture`. | **Accept non-fatal for write failures, fatal for outline-not-found.** |
| **Content append format** | [GLM]: `format!("\n{match_key}")` (raw match_key). [QWEN]: `format!("\n- [[{}]]", match_key)` (markdown link). [DEEPSEEK]: `format!("{existing}\n{match_key}")` (read-then-write). [NEMOTRON]: `content || chr(10) || $1` (SQL concat). | **Reject [QWEN]'s markdown-wikilink format** — the request says "append the note match_key", not "append a wikilink." The acceptance criteria check `outline content` contains the match_key; the simplest interpretation is raw match_key. **Reject [DEEPSEEK]'s read-then-write** — it requires an extra DB roundtrip and has a race window. | **Accept [GLM]'s SQL-side concat** (`content || $2` where `$2 = "\n{match_key}"`) — single atomic UPDATE, matches the existing capture upsert's `content || chr(10) || excluded.content` pattern. |
| **Self-attach guard** | [NEMOTRON] raises self-loop concern. [GLM] recommends guard. [DEEPSEEK] and [QWEN] don't mention it. | **Accept [GLM]/[NEMOTRON] guard.** If a caller passes the outline's own match_key as `attach_to_outline`, we'd create a self-loop `part_of(outline→outline)` and append the outline's match_key to its own content. The guard `if outline_note.id != note_id` prevents this cleanly. | **Accept self-attach guard.** |
| **Test placement** | [GLM]: new file `tests/mcp_capture.rs` with `pub` functions. [DEEPSEEK]: inline `#[cfg(test)] mod tests` in `mcp.rs`. | **Accept [GLM]'s approach** — separate test file is cleaner, matches the existing `tests/pipeline_integration.rs` pattern, and `pub` visibility on internal tool functions is acceptable for an internal crate. Making `tool_capture` and `tool_edges` `pub` is low-cost since the crate is not published. | **Accept [GLM]: new test file + pub visibility.** |
| **Wiki re-projection of outline** | [GLM], [QWEN], [NEMOTRON] agree: re-project outline after content update (non-fatal). [DEEPSEEK] doesn't mention it. | **Accept.** The existing `tool_capture` already re-projects the captured note. Re-projecting the outline after content change is consistent and ensures the wiki reflects the appended line. With `WikiConfig::default()` (wiki disabled), `materialize` is a no-op (`wiki.rs:81`), so tests are unaffected. | **Accept.** |

### Edge Cases

1. **Outline not found**: Returns `-32000` error. Note is already persisted (upsert completed before attach logic).
2. **Self-attach** (outline = captured note): Guarded — skips edge + content append, `attached_to = None`.
3. **Re-capture same note with same outline**: Edge insert is idempotent (`ON CONFLICT DO NOTHING`). Content append is **not deduped** — the match_key will be appended again. This is acceptable for v1 and consistent with the existing capture upsert's append-on-repeat behavior. If dedup is needed later, guard with a `content NOT LIKE` check.
4. **Empty string `attach_to_outline`**: `.as_str()` returns `Some("")`, which would attempt to resolve an outline with empty string. Add `.filter(|s| !s.is_empty())` to treat empty as omitted. ([NEMOTRON] raises this; valid.)
5. **Read-only mode**: The existing `read_only` guard at the top of `tool_capture` returns early before reaching attach logic — attach is correctly blocked.
6. **Concurrency**: `insert_edge_if_not_exists` uses `ON CONFLICT DO NOTHING` — safe under concurrent captures. The content append `UPDATE ... content || $2` is atomic at the row level.

---

## Verification Criteria

### Build

```sh
cargo build
```
Must compile with no errors.

### Test (requires `DATABASE_URL`)

```sh
DATABASE_URL=postgres://... cargo test --test mcp_capture
```

Tests skip cleanly (return early) when `DATABASE_URL` is not set — no `#[ignore]` needed, mirrors `tests/pipeline_integration.rs` pattern.

### Acceptance Criterion 1: Single-call creates note + edge + content update

**Test:** `test_capture_with_attach_to_outline_creates_edge_and_appends_content`

1. Seed an outline note with `entity_type="outline"`, `match_key="outline:test-offload-index"`, `content=Some("Initial outline.")`.
2. Call `tool_capture(state, json!(1), json!({ "entity_type": "note", "name": "My Fact", "lede": "A fact.", "source": "skill", "attach_to_outline": "outline:test-offload-index" }))`.
3. Assert response: `status == "created"`, `attached_to == "outline:test-offload-index"`, `match_key` and `note_id` present.
4. Assert `db::edges_for_note(&db, &outline_id)` contains a `part_of` edge with `source_note_id == new_note_id`, `target_note_id == outline_id`, `from_source == db::MANUAL_SOURCE_ID`.
5. Assert `db::find_note_by_match_key(&db, "outline:test-offload-index").content` == `"Initial outline.\n{new_match_key}"`.

### Acceptance Criterion 2: Omitted param — behavior unchanged

**Test:** `test_capture_without_attach_is_unchanged`

1. Call `tool_capture(state, json!(1), json!({ "entity_type": "note", "name": "Bare Fact", "lede": "Bare.", "source": "skill" }))`.
2. Assert response: `status == "created"`, `attached_to` is null.
3. Assert `db::edges_for_note(&db, &new_note_id)` has no `part_of` edges.

### Acceptance Criterion 3: `anansi_edges <outline> part_of` returns the new note

**Test:** `test_anansi_edges_outline_part_of_returns_new_note`

1. Seed an outline note.
2. Capture a note with `attach_to_outline`.
3. Call `tool_edges(state, json!(2), json!({ "id": outline_id, "depth": 1, "edge_type": "part_of" }))`.
4. Assert response `nodes` array contains a node with `id == new_note_id`.

### Regression Check

```sh
DATABASE_URL=postgres://... cargo test --test pipeline_integration
```
Existing tests must still pass.

### Full Test Run

```sh
DATABASE_URL=postgres://... cargo test
```

---

## Logical Consequences

### Mandatory Second-Order Review

#### 1. Sites referencing `anansi_capture` tool / `tool_capture`

| Site | Location | Decision | Rationale | Time Horizon |
|------|----------|----------|-----------|--------------|
| `handle_tools_list` tool schema | `src/mcp.rs:334-355` | **Change** — add `attach_to_outline` property | The tool's input schema must advertise the new parameter for MCP clients to discover and use it. | Immediate |
| `handle_json_rpc` dispatch | `src/mcp.rs:559` | **Keep** — no change needed | The dispatch maps `"anansi_capture" => tool_capture(...)`. The new param is read from `args` inside `tool_capture`, so the dispatch is unaffected. | Immediate |
| Skill-layer callers (anansi skill prompt) | External to this repo (skill definitions) | **Keep for now, update later** | The skill prompt should eventually document the new parameter so users know they can pass it. But this is out of scope for this card — the parameter is optional and backward-compatible. | Next sprint |
| `anansi_capture` tool description text | `src/mcp.rs:335` | **Change** — update description to mention optional outline attachment | Helps MCP clients and skill authors discover the feature. Low-risk text change. | Immediate |

#### 2. Sites referencing `part_of` edges

| Site | Location | Decision | Rationale | Time Horizon |
|------|----------|----------|-----------|--------------|
| `atomized_ingest.rs` Pass 2a | `src/atomized_ingest.rs:179-194` | **Keep** — no change | The atomized ingest pipeline creates `part_of` edges from block notes to outlines with `from_source = src.id` (the source document). Our new edges use `from_source = MANUAL_SOURCE_ID`. These coexist without conflict because the unique constraint includes `from_source`, so an atomized-sourced `part_of` and a manual-sourced `part_of` between the same pair are treated as distinct edges. This is correct: a note can be part of an outline via both an atomized source and a manual capture. | Immediate |
| `anansi_edges` (`tool_edges`) BFS | `src/mcp.rs:916-1006` | **Keep** — no change | BFS traverses both `source_note_id` and `target_note_id` via `edges_for_note`. When called on the outline with `edge_type: "part_of"`, it will discover the captured note as a neighbor regardless of edge direction. No change needed. | Immediate |
| Edge unique constraint | DB schema (`edges` table) | **Keep** — no migration | The constraint `(source_note_id, target_note_id, edge_type, from_source)` already supports manual + atomized edges coexisting. | Immediate |

#### 3. Sites referencing outline `content` field

| Site | Location | Decision | Rationale | Time Horizon |
|------|----------|----------|-----------|--------------|
| `atomized_ingest.rs` outline content update | `src/atomized_ingest.rs:127-134` | **Keep** — no change | The atomized pipeline writes outline content during ingest. Our append is additive (`content || $2`) and uses a different trigger path (MCP capture, not ingest). No conflict. | Immediate |
| `tool_update_note` | `src/mcp.rs` (anansi_update_note) | **Keep** — no change | `tool_update_note` replaces content wholesale. It's a separate tool with a different contract. Our append happens only in `tool_capture` with `attach_to_outline`. | Immediate |
| Wiki materialization | `src/wiki.rs` | **Keep** — no change | `WikiStore::materialize` is already called non-fatally in `tool_capture`. We add one more non-fatal call for the outline. The wiki is regenerated from DB state, so the appended content will appear naturally. | Immediate |

#### 4. Trace: data/logic flow end-to-end

```
Caller → MCP JSON-RPC → handle_json_rpc → tool_capture
  ├─ check_skill_source (gate: must be "skill")
  ├─ read_only check (gate: must be false)
  ├─ extract entity_type, name, lede, why, content
  ├─ compute match_key
  ├─ db::find_note_by_match_key (check existing)
  ├─ INSERT ... ON CONFLICT(match_key) DO UPDATE (upsert note)
  ├─ wiki.materialize (captured note — non-fatal, existing)
  ├─ [NEW] if attach_to_outline present:
  │    ├─ resolve outline (match_key or id)
  │    ├─ guard: skip if self-attach
  │    ├─ db::insert_edge_if_not_exists (part_of, MANUAL_SOURCE_ID)
  │    ├─ UPDATE notes SET content = content || "\n{match_key}" (outline)
  │    └─ wiki.materialize (outline — non-fatal)
  └─ return JSON-RPC response with {status, note_id, match_key, name, attached_to}
```

**Break reconciliation:** No breaks found. The new block is entirely gated on `attach_to_outline` being present. When omitted, the code path is identical to today plus one extra `null` field in the response (`attached_to: null`). Existing callers ignore unknown response keys.

#### 5. "And then what?" analysis

- **Consequence:** The outline content grows with each attached note's match_key. → **And then what?** Outlines that receive many attachments will have long content fields. → **And then what?** Wiki materialization will render larger outline pages. This is expected behavior (outlines are meant to aggregate references). No mitigation needed for v1. If performance becomes an issue, pagination or content compaction can be added later. → **Time horizon: Next quarter.**
- **Consequence:** Re-capturing the same note with the same outline appends the match_key again (duplicate line). → **And then what?** The outline content has duplicate entries. → **And then what?** Wiki pages show duplicate references. → **Mitigation:** Document in the tool description that re-capture is idempotent for the edge but not for content append. If dedup is needed, add a `WHERE content NOT LIKE '%' || $2 || '%'` guard. → **Time horizon: Next sprint (optional hardening).**
- **Consequence:** The `attached_to` response field is new. → **And then what?** Skill-layer code that parses the response must handle the new field (or ignore it). → **And then what?** Since it's `null` when omitted and a string when attached, existing parsers using `serde_json` with lenient deserialization will not break. → **Time horizon: Immediate (verify no parser breaks).**

#### 6. Consequence summary

| Consequence | Type | Horizon | Action |
|-------------|------|---------|--------|
| Outline content grows with attachments | **Intended** — amplify | Immediate | Document in tool description |
| Re-capture appends duplicate match_key | **Negative** — mitigate | Next sprint | Document; optional dedup guard |
| `attached_to: null` in response when omitted | **Intended** — neutral | Immediate | Verify no parser breaks; safe (null is additive) |
| Manual + atomized `part_of` edges coexist | **Intended** — amplify | Immediate | No action needed; unique constraint handles it |
| Skill prompt should document new param | **Intended** — amplify | Next sprint | Out of scope for this card; update skill prompt separately |
| Wiki re-projection of outline | **Intended** — neutral | Immediate | Non-fatal, no-op when wiki disabled |

#### 7. Dead UI / docs / data check

- No dead UI: the new parameter is live and functional.
- No dead docs: the tool description will be updated to mention the feature.
- No dead data: no schema changes, no orphaned fields.
- The `attached_to` response field is always present (null or string) — no conditional omission that could confuse parsers.

---

## Commit & Close-Out Checklist

1. Implement changes in `src/mcp.rs` + create `tests/mcp_capture.rs`.
2. `cargo build` — must compile.
3. `DATABASE_URL=... cargo test --test mcp_capture` — 3 tests pass.
4. `DATABASE_URL=... cargo test --test pipeline_integration` — no regression.
5. `git add -A && git commit -m "feat(mcp): add attach_to_outline option to anansi_capture"`.
6. Run close-out:
   ```sh
   bash /home/pakele/.hermes/skills/git-issue-to-kanban/scripts/epic-close.sh \
     --repo <owner/repo> --issue <N> --epic-branch <branch> --workdir <repo-root>
   ```
7. If epic-close reports "branch is not ahead of master" → commit was missing → fix and re-run.
8. After merge+close succeeds, request review against delivered master state.

---

## Consensus & Divergence

**Consensus (all 4 agents agree):**
- Only `src/mcp.rs` needs modification (no `db.rs`, no migrations, no new deps).
- `attach_to_outline` is optional; omitting it must be a no-op.
- Edge direction: `source_note_id = captured note`, `target_note_id = outline`, `edge_type = "part_of"`, `from_source = MANUAL_SOURCE_ID`.
- Outline resolution: `contains(':')` → match_key, else UUID.
- `insert_edge_if_not_exists` for idempotent edge creation.
- Reuse existing `make_context` pattern from `tests/pipeline_integration.rs`.
- Tests must pass `"source": "skill"` and skip when `DATABASE_URL` is unset.

**Divergence resolved:**
- **Error handling** ([GLM]/[DEEPSEEK] fatal vs [QWEN]/[NEMOTRON] non-fatal for outline-not-found): **Resolved → fatal for not-found, non-fatal for write failures.** The note is already saved; a missing outline is a caller error worth signaling. Write failures (edge insert, content append) are unlikely and non-fatal matches the existing `wiki.materialize` error pattern.
- **Content append format** ([GLM] raw match_key vs [QWEN] wikilink `[[match_key]]` vs [DEEPSEEK] read-then-write): **Resolved → raw match_key via SQL-side concat.** The request says "append the note match_key", not a wikilink. SQL-side `content || $2` is atomic and matches the existing capture upsert pattern.
- **Test placement** ([GLM] new file + `pub` vs [DEEPSEEK] inline `#[cfg(test)]`): **Resolved → new file + `pub` visibility.** Cleaner, matches existing pattern, low cost for internal crate.
- **Self-attach guard** ([GLM]/[NEMOTRON] guard vs [DEEPSEEK]/[QWEN] silent): **Resolved → add guard.** Prevents self-loop edge and self-content-append.
- **Empty-string handling** ([NEMOTRON] `.filter(|s| !s.is_empty())`): **Resolved → accept.** Treat empty string as omitted.

**Failed/missing sources:** None. All 4 agents completed successfully.