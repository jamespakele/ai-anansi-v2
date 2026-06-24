//! LLM semantic lint — the analytical half of the anansi-crawl loop (Build-14).
//!
//! Incrementally (notes changed since a stored watermark, capped per pass) asks
//! the configured `LlmClient` to inspect each note against its edge-neighbors for
//! four problem classes — contradictions, stale claims, under-linking, and data
//! gaps. Contradiction-flagged notes get `has_conflicts = 1` in Postgres; all
//! findings are written to a browsable `lint.md`. Entirely best-effort: any
//! per-note LLM/parse/DB error is logged and skipped.

use anyhow::Result;
use serde::Deserialize;

use crate::db::{self, DbPool, NoteRecord};
use crate::llm::{InferOpts, LlmClient};
use crate::wiki::WikiStore;

const LINT_PROMPT: &str = include_str!("../prompts/lint.txt");

#[derive(Debug, Default, serde::Serialize)]
pub struct LintReport {
    pub notes_analyzed: usize,
    pub contradictions: usize,
    pub stale: usize,
    pub under_linked: usize,
    pub gaps: usize,
    pub conflicts_flagged: usize,
    /// Notes whose LLM call errored (transient) — skipped this pass; re-linted
    /// only when the note is next edited.
    pub errors: usize,
    /// Notes whose LLM output was unparseable JSON — skipped this pass.
    pub skipped: usize,
}

/// Single-flight guard: prevents the background crawl's lint phase and the
/// on-demand `anansi_wiki_lint` MCP tool from running concurrently (which would
/// double LLM cost and race the watermark / lint.md writes).
static LINT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

// ── LLM output shape ───────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
struct RawFindings {
    #[serde(default)]
    contradictions: Vec<Contradiction>,
    #[serde(default)]
    stale: Vec<Detail>,
    #[serde(default)]
    under_linked: Vec<UnderLinked>,
    #[serde(default)]
    gaps: Vec<Gap>,
}

#[derive(Debug, Deserialize)]
struct Contradiction {
    #[serde(default)]
    with: String,
    #[serde(default)]
    detail: String,
}
#[derive(Debug, Deserialize)]
struct Detail {
    #[serde(default)]
    detail: String,
}
#[derive(Debug, Deserialize)]
struct UnderLinked {
    #[serde(default)]
    suggest: String,
    #[serde(default)]
    detail: String,
}
#[derive(Debug, Deserialize)]
struct Gap {
    #[serde(default)]
    concept: String,
    #[serde(default)]
    detail: String,
}

impl RawFindings {
    fn is_empty(&self) -> bool {
        self.contradictions.is_empty()
            && self.stale.is_empty()
            && self.under_linked.is_empty()
            && self.gaps.is_empty()
    }
}

// ── Entry point ────────────────────────────────────────────────────────────────

pub async fn run_lint(
    pool: &DbPool,
    wiki: &WikiStore,
    llm: &dyn LlmClient,
    batch_max: u64,
) -> Result<LintReport> {
    let mut report = LintReport::default();

    // Single-flight: if a lint is already running (crawl phase or another tool
    // call), skip rather than double-spend and race the watermark.
    let _guard = match LINT_LOCK.try_lock() {
        Ok(g) => g,
        Err(_) => {
            eprintln!("[lint] already running — skipping this invocation");
            return Ok(report);
        }
    };

    let state_path = wiki.root.join(".lint-state");
    // `.lint-state` holds the keyset cursor "<updated_at>\t<id>" of the last note
    // analyzed. Parse defensively; a missing/old single-field file ⇒ start fresh.
    let cursor_raw: Option<String> = std::fs::read_to_string(&state_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let cursor: Option<(&str, &str)> = cursor_raw
        .as_deref()
        .and_then(|s| s.split_once('\t'));

    let notes = db::notes_changed_since(pool, cursor, batch_max as i64).await?;
    if notes.is_empty() {
        eprintln!("[lint] nothing to lint (cursor up to date)");
        return Ok(report);
    }

    let opts = InferOpts {
        temperature: 0.2,
        max_tokens: 1500,
        json_mode: true,
    };

    let mut body = String::new();
    for note in &notes {
        // Neighbor context from edges.
        let edges = db::edges_for_note(pool, &note.id).await.unwrap_or_default();
        let mut neighbors: Vec<(NoteRecord, String)> = Vec::new();
        for e in &edges {
            let other_id = if e.source_note_id == note.id {
                &e.target_note_id
            } else {
                &e.source_note_id
            };
            if let Ok(Some(n)) = db::get_note(pool, other_id).await {
                neighbors.push((n, e.edge_type.clone()));
            }
        }

        let prompt = build_prompt(note, &neighbors);
        let raw = match llm.infer(&prompt, opts.clone()).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[lint] llm infer failed for '{}': {e}", note.name);
                report.errors += 1;
                continue;
            }
        };
        let findings = match parse_findings(&raw) {
            Some(f) => f,
            None => {
                eprintln!("[lint] malformed JSON for '{}'; skipping", note.name);
                report.skipped += 1;
                continue;
            }
        };

        report.notes_analyzed += 1;
        report.contradictions += findings.contradictions.len();
        report.stale += findings.stale.len();
        report.under_linked += findings.under_linked.len();
        report.gaps += findings.gaps.len();

        // Refresh the conflict flag for THIS analyzed note (set if contradictions,
        // clear otherwise — we just re-analyzed it).
        let has_contra = !findings.contradictions.is_empty();
        if db::set_conflict(pool, &note.id, has_contra).await.is_ok() && has_contra {
            report.conflicts_flagged += 1;
        }

        render_note_findings(&mut body, note, &findings);
    }

    // Append this run's section to lint.md (a standing journal — incremental
    // passes accumulate rather than overwrite). Advance the keyset cursor to the
    // newest note in the batch, guaranteeing forward progress even past notes
    // that errored this pass (they re-lint when next edited; surfaced via
    // report.errors/skipped, not silent).
    let _ = std::fs::create_dir_all(&wiki.root);
    append_report(&wiki.root.join("lint.md"), &report, &body);
    if let Some(last) = notes.last() {
        let line = format!("{}\t{}", last.updated_at, last.id);
        if let Err(e) = std::fs::write(&state_path, &line) {
            // If we can't persist the cursor, the same batch re-lints next pass —
            // log loudly so the wasted cost is visible (e.g. read-only mount).
            eprintln!("[lint] FAILED to write cursor {}: {e} — batch will re-lint", state_path.display());
        }
    }

    Ok(report)
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn build_prompt(note: &NoteRecord, neighbors: &[(NoteRecord, String)]) -> String {
    let note_block = format!(
        "Name: {}\nType: {}\nLede: {}\nWhy: {}\nContent:\n{}",
        note.name,
        note.entity_type,
        note.lede.as_deref().unwrap_or(""),
        note.why.as_deref().unwrap_or(""),
        note.content.as_deref().unwrap_or(""),
    );
    let neighbors_block = if neighbors.is_empty() {
        "(none — this note has no edges)".to_string()
    } else {
        neighbors
            .iter()
            .map(|(n, edge)| {
                format!(
                    "- [{}] {} ({}): {}",
                    edge,
                    n.name,
                    n.entity_type,
                    n.lede.as_deref().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    LINT_PROMPT
        .replace("{{NOTE}}", &note_block)
        .replace("{{NEIGHBORS}}", &neighbors_block)
}

/// Parse the LLM's JSON, tolerating ```json fences, leading/trailing prose, and
/// trailing braces by extracting the FIRST balanced `{...}` object. Returns None
/// on unrecoverable malformed output.
fn parse_findings(raw: &str) -> Option<RawFindings> {
    let json = extract_first_json_object(raw)?;
    serde_json::from_str::<RawFindings>(json).ok()
}

/// First balanced `{...}` object, respecting string literals and escapes — so a
/// stray `}` in trailing prose or a second object doesn't corrupt the slice.
fn extract_first_json_object(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    let start = s.find('{')?;
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escaped = false;
    for i in start..bytes.len() {
        let c = bytes[i];
        if in_str {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_str = false;
            }
        } else {
            match c {
                b'"' => in_str = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(&s[start..=i]);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn render_note_findings(out: &mut String, note: &NoteRecord, f: &RawFindings) {
    if f.is_empty() {
        return;
    }
    out.push_str(&format!("### {} ({})\n\n", note.name, note.entity_type));
    for c in &f.contradictions {
        out.push_str(&format!("- ⚠️ **Contradiction** with *{}* — {}\n", c.with, c.detail));
    }
    for s in &f.stale {
        out.push_str(&format!("- 🕰️ **Stale** — {}\n", s.detail));
    }
    for u in &f.under_linked {
        out.push_str(&format!("- 🔗 **Under-linked** → *{}* — {}\n", u.suggest, u.detail));
    }
    for g in &f.gaps {
        out.push_str(&format!("- 🕳️ **Gap**: *{}* — {}\n", g.concept, g.detail));
    }
    out.push('\n');
}

/// Render one run's section (pure — testable). Each lint pass appends one of
/// these to `lint.md` so the report accumulates rather than overwriting.
fn render_run_section(report: &LintReport, body: &str) -> String {
    let date = chrono::Utc::now().format("%Y-%m-%d");
    let mut out = String::new();
    out.push_str(&format!(
        "## Lint run [{date}] — {} analyzed ({} errored, {} unparseable): \
         {} contradictions, {} stale, {} under-linked, {} gaps\n\n",
        report.notes_analyzed, report.errors, report.skipped,
        report.contradictions, report.stale, report.under_linked, report.gaps
    ));
    if body.is_empty() {
        out.push_str("_No findings in this batch._\n\n");
    } else {
        out.push_str(body);
    }
    out
}

/// Append this run's section to `lint.md`, writing the title header once.
fn append_report(path: &std::path::Path, report: &LintReport, body: &str) {
    use std::io::Write as _;
    let need_header = !path.exists();
    let section = render_run_section(report, body);
    match std::fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut f) => {
            if need_header {
                let _ = f.write_all("# Anansi LLM-Wiki — Lint Report\n\n".as_bytes());
            }
            if let Err(e) = f.write_all(section.as_bytes()) {
                eprintln!("[lint] failed to append lint.md: {e}");
            }
        }
        Err(e) => eprintln!("[lint] failed to open lint.md: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_well_formed_json() {
        let raw = r#"{"contradictions":[{"with":"A","detail":"x"}],"stale":[],"under_linked":[],"gaps":[{"concept":"C","detail":"y"}]}"#;
        let f = parse_findings(raw).expect("should parse");
        assert_eq!(f.contradictions.len(), 1);
        assert_eq!(f.gaps.len(), 1);
        assert!(f.stale.is_empty());
    }

    #[test]
    fn parses_fenced_json_with_prose() {
        let raw = "Here you go:\n```json\n{\"contradictions\":[],\"stale\":[{\"detail\":\"old\"}]}\n```";
        let f = parse_findings(raw).expect("should parse fenced");
        assert_eq!(f.stale.len(), 1);
        assert!(f.contradictions.is_empty());
        assert!(f.is_empty() == false);
    }

    #[test]
    fn malformed_json_returns_none() {
        assert!(parse_findings("not json at all").is_none());
        assert!(parse_findings("{ broken: ").is_none());
    }

    #[test]
    fn extracts_first_object_ignoring_trailing_prose_and_braces() {
        // Trailing prose containing a stray brace must not corrupt the slice.
        let raw = r#"{"stale":[{"detail":"x }"}]} and then the model rambled } here"#;
        let f = parse_findings(raw).expect("should parse first object");
        assert_eq!(f.stale.len(), 1);
        // A second object is ignored (first balanced object wins).
        let two = r#"{"gaps":[]}{"gaps":[{"concept":"C","detail":"d"}]}"#;
        let f2 = parse_findings(two).expect("first object");
        assert!(f2.gaps.is_empty());
    }

    #[test]
    fn empty_findings_render_nothing() {
        let note = sample_note();
        let mut body = String::new();
        render_note_findings(&mut body, &note, &RawFindings::default());
        assert!(body.is_empty());
    }

    #[test]
    fn report_renders_summary_and_findings() {
        let note = sample_note();
        let f = parse_findings(
            r#"{"contradictions":[{"with":"Other","detail":"conflicts"}]}"#,
        )
        .unwrap();
        let mut body = String::new();
        render_note_findings(&mut body, &note, &f);
        let report = LintReport { notes_analyzed: 1, contradictions: 1, ..Default::default() };
        let md = render_run_section(&report, &body);
        assert!(md.contains("Lint run"));
        assert!(md.contains("1 analyzed"));
        assert!(md.contains("Contradiction"));
        assert!(md.contains("Other"));
    }

    fn sample_note() -> NoteRecord {
        NoteRecord {
            id: "a".into(),
            entity_type: "concept".into(),
            name: "Test".into(),
            match_key: "concept:test".into(),
            lede: Some("l".into()),
            why: None,
            content: Some("c".into()),
            has_conflicts: 0,
            conflicts_updated_at: None,
            merge_category: "entity".into(),
            created_from: "s".into(),
            source_count: 1,
            created_at: "2026-06-24T00:00:00Z".into(),
            updated_at: "2026-06-24T00:00:00Z".into(),
        }
    }
}
