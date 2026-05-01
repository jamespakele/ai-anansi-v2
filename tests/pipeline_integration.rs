use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use anansi2::llm::{InferOpts, LlmClient};
use anansi2::pipeline::{IngestContext, ingest};

struct MockLlm {
    pass1_response: String,
    pass3_response: String,
    pass4_response: String,
    call_count: Arc<AtomicUsize>,
    json_call_count: Arc<AtomicUsize>,
    total_leaves: usize,  // after this many json calls, it's pass4
}

#[async_trait::async_trait]
impl LlmClient for MockLlm {
    async fn infer(&self, _prompt: &str, opts: InferOpts) -> anyhow::Result<String> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        if opts.json_mode {
            let n = self.json_call_count.fetch_add(1, Ordering::SeqCst);
            if n < self.total_leaves {
                Ok(self.pass3_response.clone())
            } else {
                Ok(self.pass4_response.clone())
            }
        } else {
            Ok(self.pass1_response.clone())
        }
    }

    async fn ping(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set"))
}

fn make_mock_llm(
    pass1: &str,
    pass3: &str,
    pass4: &str,
    total_leaves: usize,
) -> (Box<MockLlm>, Arc<AtomicUsize>) {
    let count = Arc::new(AtomicUsize::new(0));
    let json_count = Arc::new(AtomicUsize::new(0));
    let llm = Box::new(MockLlm {
        pass1_response: pass1.to_string(),
        pass3_response: pass3.to_string(),
        pass4_response: pass4.to_string(),
        call_count: count.clone(),
        json_call_count: json_count,
        total_leaves,
    });
    (llm, count)
}

async fn make_context(
    root: &Path,
    llm: Box<dyn LlmClient>,
) -> IngestContext {
    use anansi2::config::{Config, LlmConfig, InferSettings, PathsConfig, ServerConfig, PipelineConfig};
    use anansi2::db::open_and_migrate;
    use anansi2::rules::RuleRegistry;
    use anansi2::template::TemplateRegistry;
    use anansi2::vault::Vault;
    use std::path::PathBuf;

    let web_dir = root.join("web");
    std::fs::create_dir_all(&web_dir).unwrap();

    let db_path = root.join("test.db");
    let db = open_and_migrate(&db_path).await.unwrap();

    let md = manifest_dir();
    let templates = TemplateRegistry::load(&md.join("templates")).unwrap();
    let rules = RuleRegistry::load(&md.join("%Rules")).unwrap();
    let vault = Vault::new(root.to_path_buf(), Path::new("web"));

    let config = Config {
        paths: PathsConfig {
            web_dir: PathBuf::from("web"),
            rules_dir: PathBuf::from("%Rules"),
            templates_dir: PathBuf::from("templates"),
            db_file: PathBuf::from("test.db"),
        },
        llm: LlmConfig {
            backend: "ollama".to_string(),
            url: "http://localhost:11434".to_string(),
            model: "qwen2.5:14b".to_string(),
            n_ctx: 16384,
            timeout_s: 60,
            decomposition: InferSettings { temperature: 0.2, max_tokens: 4096, json_mode: false },
            extraction: InferSettings { temperature: 0.2, max_tokens: 4096, json_mode: true },
            synthesis: InferSettings { temperature: 0.2, max_tokens: 4096, json_mode: true },
            gemini: None,
            openrouter: None,
        },
        server: ServerConfig::default(),
        pipeline: PipelineConfig { mode: None },
    };

    IngestContext {
        anansi_root: root.to_path_buf(),
        config,
        vault,
        db,
        templates,
        rules,
        llm: Some(llm),
    }
}

fn pass3_json_for(name: &str, entity_type: &str) -> String {
    serde_json::json!({
        "fields": {
            "name": name,
            "contact_email": "[not mentioned]",
            "contact_phone": "[not mentioned]",
            "summary": format!("A {entity_type} named {name}.")
        },
        "roster": {},
        "lede": format!("{name} is a {entity_type}."),
        "why": format!("{name} is a {entity_type} involved in various activities."),
        "tags": [entity_type],
        "entities": []
    })
    .to_string()
}

#[tokio::test]
async fn test_ingest_basic() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Create source file with 5 leaves
    let source_content = r#"---
title: Test Workshop 2026-01-30
source_type: meeting_summary
---
This is a test workshop transcript. Ian Kitajima from PICHTR presented.
PICHTR is an organization. Sovereign AI was discussed. Context was important.
"#;
    let source_path = root.join("test-workshop.md");
    std::fs::write(&source_path, source_content).unwrap();

    // TOC with 3 pure-atomic + 1 container + 1 source-bound = 5 leaves
    let pass1_toc = "1.1 Ian Kitajima [person] | hint: see attendee list\n\
                     1.2 Sovereign AI [note] | hint: background section\n\
                     1.3 AI Research [topic] | hint: research section\n\
                     2.1 PICHTR [organization] | hint: org section\n\
                     3.1 Workshop Discussion [context] | hint: main discussion\n";

    // Use a generic pass3 JSON that works for any entity type
    let pass3 = pass3_json_for("Test Entity", "person");
    let pass4 = "[]";

    let (llm, _count) = make_mock_llm(pass1_toc, &pass3, pass4, 5);
    let ctx = make_context(root, llm).await;

    let result = ingest(&ctx, &source_path).await.unwrap();

    // Assert: 1 source row
    let source_rec = anansi2::db::get_source(&ctx.db, &result.source_id)
        .await
        .unwrap()
        .expect("source should exist");
    assert_eq!(source_rec.source_path, source_path.to_string_lossy().to_string());

    // Assert: outline note exists
    let outline_note = anansi2::db::get_note(&ctx.db, &result.outline_note_id)
        .await
        .unwrap()
        .expect("outline note should exist");
    assert_eq!(outline_note.entity_type, "outline");

    // Assert: at least 5 leaf notes created + 1 outline = 6 total (outline is already 1)
    assert!(
        result.atomic_notes_created >= 5,
        "expected at least 5 leaf notes created, got {}",
        result.atomic_notes_created
    );

    // Assert: at least 5 contains edges
    assert!(
        result.edges_created >= 5,
        "expected at least 5 edges, got {}",
        result.edges_created
    );

    // Assert: pass1 was called
    assert!(result.pass1_llm_called);

    // Assert: outline file exists on disk
    let outline_path = root.join("web").join("test-workshop.outline.md");
    assert!(outline_path.exists(), "outline file should exist at {:?}", outline_path);
}

#[tokio::test]
async fn test_pass1_skipped_when_toc_present() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Source with valid anansi_toc frontmatter (2 leaves)
    let source_content = "---\ntitle: Preprocessed Doc\nsource_type: container\nanansi_toc: |\n  1.1 Alice Smith [person]\n  1.2 Tech Corp [organization]\n---\nBody content here.\n";
    let source_path = root.join("preprocessed.md");
    std::fs::write(&source_path, source_content).unwrap();

    let pass3 = pass3_json_for("Alice Smith", "person");
    let pass4 = "[]";

    let (llm, count) = make_mock_llm("SHOULD_NOT_BE_CALLED", &pass3, pass4, 2);
    let ctx = make_context(root, llm).await;

    let result = ingest(&ctx, &source_path).await.unwrap();

    // Pass 1 should NOT have been called
    assert!(!result.pass1_llm_called, "Pass 1 LLM should not have been called");

    // LLM call count: 2 leaves (pass3) + 1 (pass4) = 3
    let calls = count.load(Ordering::SeqCst);
    assert_eq!(calls, 3, "expected 3 LLM calls (2 pass3 + 1 pass4), got {calls}");
}

#[tokio::test]
async fn test_invalid_toc_falls_back_to_pass1() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Source with malformed anansi_toc (duplicate address)
    let source_content = "---\ntitle: Bad TOC Doc\nsource_type: container\nanansi_toc: |\n  1.1 Alice Smith [person]\n  1.1 Bob Jones [person]\n---\nBody content.\n";
    let source_path = root.join("bad-toc.md");
    std::fs::write(&source_path, source_content).unwrap();

    // Pass 1 returns a valid single-leaf TOC
    let pass1_toc = "1.1 Alice Smith [person]\n";
    let pass3 = pass3_json_for("Alice Smith", "person");
    let pass4 = "[]";

    let (llm, _count) = make_mock_llm(pass1_toc, &pass3, pass4, 1);
    let ctx = make_context(root, llm).await;

    let result = ingest(&ctx, &source_path).await.unwrap();

    // Pass 1 LLM SHOULD have been called due to invalid TOC
    assert!(result.pass1_llm_called, "Pass 1 should have been called due to invalid TOC");
}
