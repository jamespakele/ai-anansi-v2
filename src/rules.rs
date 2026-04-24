use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

pub struct RuleRegistry {
    rules: HashMap<String, String>,
}

impl RuleRegistry {
    pub fn load(rules_dir: &Path) -> Result<Self> {
        let mut rules = HashMap::new();

        let entries = std::fs::read_dir(rules_dir)
            .with_context(|| format!("reading rules dir: {}", rules_dir.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .trim_start_matches('%')
                .to_string();

            if stem.is_empty() {
                continue;
            }

            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("reading rule file: {}", path.display()))?;

            // Strip YAML frontmatter if present
            let body = strip_frontmatter(&content);
            rules.insert(stem, body.to_string());
        }

        Ok(Self { rules })
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.rules.get(name).map(|s| s.as_str())
    }
}

fn strip_frontmatter(content: &str) -> &str {
    let rest = match content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n")) {
        Some(r) => r,
        None => return content,
    };
    match rest.split_once("\n---\n").or_else(|| rest.split_once("\n---\r\n")) {
        Some((_, after)) => after.trim_start_matches('\n'),
        None => content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn rules_dir() -> PathBuf {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        PathBuf::from(manifest).join("%Rules")
    }

    #[test]
    fn loads_all_four_rules() {
        let dir = rules_dir();
        let registry = RuleRegistry::load(&dir).expect("should load rules");
        for name in ["Atomicity", "Downstream-Flow", "Merge-Strategy", "Template-Schema"] {
            let body = registry.get(name);
            assert!(body.is_some(), "rule '{name}' not found");
            assert!(!body.unwrap().is_empty(), "rule '{name}' body is empty");
        }
    }

    #[test]
    fn rule_body_excludes_frontmatter() {
        let dir = rules_dir();
        let registry = RuleRegistry::load(&dir).expect("should load rules");
        let body = registry.get("Atomicity").unwrap();
        assert!(!body.starts_with("---"), "frontmatter should be stripped");
        assert!(body.contains("Rule 1"), "body should contain rule content");
    }
}
