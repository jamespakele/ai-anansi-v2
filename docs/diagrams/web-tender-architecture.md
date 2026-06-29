# Web Tender — Architecture Diagrams

## 1. System Architecture

How the Web Tender fits alongside existing watchers, the database, rules files, and skills.

```mermaid
flowchart TB
    subgraph Config["anansi.toml"]
        TC["[tender]\nenabled = true\ninterval_secs = 3600\nbatch_size = 100\ndry_run = true"]
    end

    subgraph Server["anansi server (cmd_serve)"]
        direction TB
        QW["queue::run_queue_watcher"]
        IW["inbox::run_inbox_watcher"]
        CW["crawl::run_crawl_watcher"]
        TW["tender::run_tender_watcher"]
    end

    subgraph Tender["src/tender.rs"]
        direction TB
        LOOP["Sleep interval_secs\n→ run_tender_pass()"]
        CHECKS["9 maintenance checks"]
        REPORT["TenderReport\n(notes, edges, flags, errors)"]
    end

    subgraph DB["PostgreSQL"]
        NOTES["notes"]
        EDGES["edges"]
        SOURCES["sources"]
        CONTRIB["source_contributions"]
        EMBED["note_embeddings"]
        QUEUE["tender_queue ⭐ NEW"]
    end

    subgraph Rules["%Rules/ (markdown)"]
        NORM["normalization.md"]
        DEDUP["deduplication.md"]
        INFER["edge-inference.md"]
        TMAP["template-mapping.md"]
    end

    subgraph Skills["llm/plugins/.../skills/"]
        WT["web-tender/SKILL.md\n--dry-run | --apply | status"]
        WTA["web-tender-audit/SKILL.md\n--resolve | --dismiss"]
    end

    Config -->|"tender.enabled = true"| TW
    TW --> LOOP
    LOOP --> CHECKS
    CHECKS -->|"reads rules"| Rules
    CHECKS -->|"queries + mutations"| DB
    CHECKS --> REPORT
    Skills -->|"invoke via agent"| TW
```

## 2. Check Priority & Flow

The 9 checks in execution order, color-coded by auto-fix vs flag-to-queue.

```mermaid
flowchart LR
    subgraph Legend["Check Categories"]
        direction TB
        AF["🟢 Auto-fix (high confidence)"]
        FL["🟡 Flag to queue (medium/low confidence)"]
    end

    subgraph Pass["Single Tender Pass"]
        START(["Sleep interval"]) --> BATCH["Fetch next batch\n(up to batch_size notes\nordered by updated_at ASC)"]
        BATCH --> C1
    end

    subgraph Checks["9 Checks in Priority Order"]
        direction TB
        C1["1. Edge integrity"]:::autofix
        C2["2. Duplicate edges"]:::autofix
        C3["3. Wikilink validity"]:::autofix
        C4["4. Exact deduplication"]:::autofix
        C5["5. Conflicting edges"]:::flag
        C6["6. Orphan notes"]:::flag
        C7["7. Stale/empty notes"]:::flag
        C8["8. Circular references"]:::flag
        C9["9. Type consistency"]:::flag
    end

    C1 --> C2 --> C3 --> C4 --> C5 --> C6 --> C7 --> C8 --> C9
    C9 --> DONE["Log TenderReport\n→ sleep again"]

    classDef autofix fill:#d4edda,stroke:#28a745,color:#155724
    classDef flag fill:#fff3cd,stroke:#ffc107,color:#856404
```

## 3. Per-Check Behavior

What each check does internally, with transaction and dedup protection.

```mermaid
flowchart TB
    subgraph AutoFix["🟢 Auto-fix Checks"]
        direction TB
        EI["Edge Integrity\n──\nFind edges where source or\ntarget note is missing\n→ DELETE dangling edge"]
        DE["Duplicate Edges\n──\nFind identical (source, target, type)\nrows via GROUP BY/HAVING\n→ DELETE extras, keep lowest ID"]
        WV["Wikilink Validity\n──\nScan content for [[wikilinks]]\nvia regex → verify target exists\n→ Remove broken [[links]] from content"]
        ED["Exact Deduplication\n──\nFind notes with same\n(normalized_name, entity_type)\n→ Merge: union why/content,\nre-point edges + contributions,\nDELETE duplicate"]
    end

    subgraph FlagOnly["🟡 Flag-only Checks"]
        direction TB
        CE["Conflicting Edges\n──\nFind contradictory edge types\nbetween same pair\n(works_at vs competitor_of)\n→ INSERT tender_queue flag"]
        ON["Orphan Notes\n──\nLEFT JOIN edges → find notes\nwith zero incoming/outgoing edges\n→ INSERT tender_queue flag"]
        SN["Stale/Empty Notes\n──\nFind notes where why AND\ncontent are NULL or empty\n→ INSERT tender_queue flag"]
        CR["Circular References\n──\nRecursive CTE to detect\nedge cycles of any length\n→ INSERT tender_queue flag"]
        TC["Type Consistency\n──\nValidate edge types match\nentity types (reports_to\nmust be person→person)\n→ INSERT tender_queue flag"]
    end

    subgraph Queue["tender_queue table"]
        QDIR["INSERT with:\n- category (broken_edge, dedup, etc.)\n- severity (low/medium/high/info)\n- match_key + related_keys\n- description + confidence\n- status = 'open'"]
    end

    subgraph Dedup["Dedup Protection"]
        DP["Before INSERT:\ncheck for existing open flag\nwith same category + match_key\n→ UPDATE updated_at instead\nof duplicate insert"]
    end

    FlagOnly --> Queue
    Queue --> Dedup
```

## 4. Sequence Diagram — Full Tender Pass Lifecycle

The complete flow from config load through batch processing to queue inserts.

```mermaid
sequenceDiagram
    participant C as Config (anansi.toml)
    participant M as main.rs (cmd_serve)
    participant T as tender.rs
    participant D as db.rs
    participant P as PostgreSQL
    participant R as %Rules/

    C->>M: tender.enabled = true
    M->>T: tokio::spawn(run_tender_watcher)
    T->>T: sleep(interval_secs)

    loop Every interval_secs
        T->>R: load rules files
        T->>D: get_notes_batch(batch_size, offset)
        D->>P: SELECT * FROM notes ORDER BY updated_at ASC LIMIT $1 OFFSET $2
        P-->>D: batch of notes
        D-->>T: Vec<NoteRecord>

        par Check 1: Edge Integrity
            T->>D: get_dangling_edges()
            D->>P: SELECT edges WHERE source/target note missing
            P-->>D: dangling edges
            alt dry_run = false
                T->>D: remove_dangling_edge(id)
                D->>P: DELETE FROM edges WHERE id = $1
            end
        and Check 2: Duplicate Edges
            T->>D: get_duplicate_edges()
            D->>P: GROUP BY/HAVING COUNT > 1
            P-->>D: duplicate groups
            alt dry_run = false
                T->>D: remove_duplicate_edge(id)
            end
        and Check 3-4: Wikilinks + Exact Dedup
            T->>D: check_wikilink_validity()
            T->>D: merge_duplicate_notes() ⚡ transaction
        and Check 5-9: Flag to Queue
            T->>D: get_conflicting_edges()
            T->>D: get_orphan_notes()
            T->>D: get_stale_notes()
            T->>D: get_circular_refs()
            T->>D: get_type_violations()
            alt dry_run = false
                T->>D: insert_or_update_flag() ⚡ dedup check
                D->>P: INSERT INTO tender_queue
            end
        end

        T->>T: build TenderReport
        T->>T: log summary
        T->>T: sleep(interval_secs)
    end
```

## 5. Phase 2 — Deferred Checks

The 6 remaining checks scoped for the next build.

```mermaid
flowchart LR
    subgraph Deferred["📋 Phase 2 — Deferred Checks"]
        direction TB
        IOS["Index/Outline Sync\n──\nVerify index.md + per-type outlines\nmatch DB state\n→ add missing, remove stale"]
        NR["Normalization Rules\n──\nApply %Rules/normalization.md\n(name casing, punctuation, slugs)\n→ update note fields"]
        TA["Template Alignment\n──\nCompare note fields against\ncurrent entity_type template\n→ re-atomize via sb-atomize"]
        SI["Source Integrity\n──\nVerify note.source_id exists\nin sources table\n→ flag orphans"]
        EI2["Edge Inference\n──\nScan content for known entity\nmentions by name/match_key\n→ suggest missing edges"]
        SD["Semantic Deduplication\n──\nCosine similarity on embeddings\n→ flag near-duplicates\nabove 0.92 threshold"]
    end

    IOS --> NR --> TA --> SI --> EI2 --> SD
```
