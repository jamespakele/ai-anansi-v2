# HANDOFF: Cross-Encoder Reranker for Semantic Search

**Status:** Future feature — not implemented  
**Date:** 2026-06-22  
**Context:** Discussed during embedding model evaluation; decided to stick with `gemini-embedding-2` at 768d. The reranker was identified as a higher-impact improvement than switching embedding models.

---

## Problem

Anansi's semantic search (`anansi_search_semantic`) uses a two-stage approach:

1. **pgvector HNSW** — approximate nearest-neighbor search via cosine distance on `gemini-embedding-2` vectors (768d)
2. **Return top-N** — results ranked purely by vector similarity

The weakness: cosine similarity on embeddings is a **bi-encoder** approach — the query and each document are encoded independently, then compared. The model never sees the query and document together. This means:

- Synonyms and paraphrases can be missed
- The model can't weigh which parts of a document are relevant to the query
- Short queries against long documents perform poorly

---

## Solution: Cross-Encoder Reranker

A **cross-encoder** reads the query and document *together* and outputs a relevance score. The architecture:

```
Query → [pgvector HNSW] → top-50 candidates → [Cross-Encoder Reranker] → re-ranked top-10
         (fast, approximate)                    (slow, precise)
```

### Why this works

- Cross-encoders achieve 5-15% improvement over bi-encoders on retrieval benchmarks
- The two-stage approach keeps latency low: pgvector prunes to 50 candidates in ~5ms, the reranker scores only those 50
- The reranker is model-agnostic — it works on top of any embedding model

---

## Implementation Plan

### Stage 1: Choose a reranker model

Options (all available via OpenRouter or self-hosted):

| Model | Provider | Notes |
|---|---|---|
| `cohere/rerank-english-v3.0` | Cohere / OpenRouter | Best-in-class, dedicated reranker API |
| `voyage/voyage-rerank-2` | Voyage AI / OpenRouter | Strong, same provider as voyage-3 embeddings |
| `jina/jina-reranker-v2-base-multilingual` | Jina AI | Open-source, can self-host |
| `BAAI/bge-reranker-v2-m3` | BAAI / Ollama | Open-source, 568M params, multilingual |

**Recommendation:** Start with `cohere/rerank-english-v3.0` via OpenRouter — it has the simplest API and best performance. Fall back to `BAAI/bge-reranker-v2-m3` via Ollama for zero-cost self-hosting.

### Stage 2: Add reranker to the search pipeline

In `src/mcp.rs`, modify `tool_search_semantic`:

```rust
async fn tool_search_semantic(state: McpState, id: Value, args: Value) -> Json<Value> {
    // ... existing embedding + pgvector search ...

    // Fetch top-50 candidates (instead of top-10)
    let candidates = pgvector_search(query_embedding, 50).await?;

    // NEW: Rerank with cross-encoder
    let reranked = if ctx.config.reranker.enabled {
        rerank(&ctx.config.reranker, &query, &candidates).await?
    } else {
        candidates  // fallback to embedding-only ranking
    };

    // Return top-10 after reranking
    json_rpc_ok(id, format_results(&reranked[..10]))
}
```

### Stage 3: Reranker module

New file: `src/reranker.rs`

```rust
pub struct RerankerConfig {
    pub enabled: bool,
    pub provider: RerankerProvider,  // OpenRouter | Ollama
    pub model: String,               // "cohere/rerank-english-v3.0"
    pub api_key: Option<String>,
    pub ollama_url: Option<String>,
    pub top_n: usize,                // how many candidates to rerank (default 50)
}

pub async fn rerank(
    config: &RerankerConfig,
    query: &str,
    candidates: &[SearchCandidate],
) -> Result<Vec<ScoredCandidate>> {
    match config.provider {
        RerankerProvider::OpenRouter => rerank_openrouter(config, query, candidates).await,
        RerankerProvider::Ollama => rerank_ollama(config, query, candidates).await,
    }
}
```

### Stage 4: Configuration

Add to `anansi.toml`:

```toml
[reranker]
enabled = false              # opt-in; disabled by default
provider = "openrouter"      # "openrouter" | "ollama"
model = "cohere/rerank-english-v3.0"
# api_key = "$OPENROUTER_API_KEY"  # or read from env
top_n = 50                   # candidates to rerank per query
```

### Stage 5: API integration

**Cohere rerank API (via OpenRouter):**

```
POST https://openrouter.ai/api/v1/rerank
Authorization: Bearer $OPENROUTER_API_KEY

{
  "model": "cohere/rerank-english-v3.0",
  "query": "What did James say about the Waiʻanae Summit?",
  "documents": [
    "Meeting notes from May 15...",
    "Email thread about venue...",
    ...
  ],
  "top_n": 10
}
```

Response:
```json
{
  "results": [
    {"index": 3, "relevance_score": 0.98},
    {"index": 0, "relevance_score": 0.87},
    ...
  ]
}
```

---

## Cost / Performance Estimates

| Metric | Without Reranker | With Reranker |
|---|---|---|
| pgvector query | ~5ms | ~5ms |
| Reranker API call | — | ~200-500ms |
| Total latency | ~5ms | ~200-500ms |
| Cost per query | $0 (Gemini free tier) | ~$0.0001 (Cohere via OpenRouter) |
| Retrieval quality | Baseline | +5-15% |

The latency increase is acceptable for a single-user knowledge base. The reranker call is a single HTTP request — no batching complexity needed at this scale.

---

## When to Implement

This is a **nice-to-have**, not a blocker. The current `gemini-embedding-2` + pgvector HNSW setup is solid for a personal knowledge base. Consider implementing when:

1. Search result quality becomes a noticeable pain point
2. The vault grows large enough that top-10 results start including irrelevant notes
3. You're already making other changes to the search pipeline

---

## Related

- `src/embed.rs` — current embedding implementation (Gemini, 768d)
- `src/mcp.rs` — `tool_search_semantic` (line ~1875)
- `migrations/0001_schema.sql` — `note_embeddings` table, HNSW index
- `docs/anansi-v2-build-03-interfaces.md` — MCP tool specifications