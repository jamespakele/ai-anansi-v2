use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum MergeStrategy {
    PureAtomic,
    Container,
    SourceBound,
}

impl<'de> Deserialize<'de> for MergeStrategy {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        match s.as_str() {
            "pure_atomic" => Ok(MergeStrategy::PureAtomic),
            "container" => Ok(MergeStrategy::Container),
            "source_bound" => Ok(MergeStrategy::SourceBound),
            other => Err(serde::de::Error::custom(format!("unknown merge_strategy: {other}"))),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldDef {
    #[serde(rename = "type", default)]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourceHint {
    pub hint: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RosterSection {
    pub source_field: String,
    pub render_as: String,
    pub row_format: String,
    #[serde(default)]
    pub dedupe_by: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FieldBlock {
    pub field: String,
    pub description: String,
    pub format: Option<String>,
    pub constraints: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TemplateFrontmatter {
    entity_type: String,
    #[serde(default)]
    atomic: bool,
    merge_strategy: MergeStrategy,
    #[serde(default)]
    template_version: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    identity_fields: HashMap<String, FieldDef>,
    #[serde(default)]
    sources: HashMap<String, SourceHint>,
    #[serde(default)]
    roster_sections: HashMap<String, RosterSection>,
}

#[derive(Debug, Clone)]
pub struct Template {
    pub entity_type: String,
    pub atomic: bool,
    pub merge_strategy: MergeStrategy,
    pub template_version: String,
    pub description: String,
    pub identity_fields: HashMap<String, FieldDef>,
    pub sources: HashMap<String, SourceHint>,
    pub roster_sections: HashMap<String, RosterSection>,
    pub field_blocks: Vec<FieldBlock>,
    pub body: String,
}

impl Template {
    pub fn render_body(&self, fields: &HashMap<String, String>) -> String {
        static PLACEHOLDER: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"\{\{(\w+)\}\}").expect("valid regex"));
        PLACEHOLDER
            .replace_all(&self.body, |caps: &regex::Captures| {
                let key = &caps[1];
                fields.get(key).map(|v| v.as_str()).unwrap_or("")
            })
            .into_owned()
    }
}

pub struct TemplateRegistry {
    templates: HashMap<String, Template>,
}

impl TemplateRegistry {
    pub fn load(templates_dir: &Path) -> Result<Self> {
        let mut templates = HashMap::new();

        let entries = std::fs::read_dir(templates_dir)
            .with_context(|| format!("reading templates dir: {}", templates_dir.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("reading template: {}", path.display()))?;

            let template = parse_template(&content)
                .with_context(|| format!("parsing template: {}", path.display()))?;

            templates.insert(template.entity_type.clone(), template);
        }

        Ok(Self { templates })
    }

    pub fn get(&self, entity_type: &str) -> Option<&Template> {
        self.templates.get(entity_type)
    }

    pub fn atomic_types(&self) -> Vec<&str> {
        let mut types: Vec<&str> = self.templates.values()
            .filter(|t| t.atomic)
            .map(|t| t.entity_type.as_str())
            .collect();
        types.sort();
        types
    }

    pub fn source_types(&self) -> Vec<&str> {
        let mut types: Vec<&str> = self.templates.values()
            .filter(|t| !t.atomic)
            .map(|t| t.entity_type.as_str())
            .collect();
        types.sort();
        types
    }

    pub fn all_entity_types(&self) -> Vec<&str> {
        let mut types: Vec<&str> = self.templates.keys().map(|s| s.as_str()).collect();
        types.sort();
        types
    }
}

fn parse_template(content: &str) -> Result<Template> {
    // Extract YAML frontmatter between first --- pair
    let rest = content.strip_prefix("---\n").unwrap_or(content.strip_prefix("---\r\n").unwrap_or(content));
    let (fm_str, after_fm) = rest
        .split_once("\n---\n")
        .or_else(|| rest.split_once("\n---\r\n"))
        .with_context(|| "could not find closing --- in template")?;

    let fm: TemplateFrontmatter = serde_yaml::from_str(fm_str)
        .with_context(|| "parsing template frontmatter YAML")?;

    // Parse %% field blocks
    let field_blocks = parse_field_blocks(after_fm);

    // Body is everything after the last %% block
    let body = extract_body(after_fm);

    Ok(Template {
        entity_type: fm.entity_type,
        atomic: fm.atomic,
        merge_strategy: fm.merge_strategy,
        template_version: fm.template_version,
        description: fm.description,
        identity_fields: fm.identity_fields,
        sources: fm.sources,
        roster_sections: fm.roster_sections,
        field_blocks,
        body,
    })
}

fn parse_field_blocks(content: &str) -> Vec<FieldBlock> {
    let mut blocks = Vec::new();
    let mut remaining = content;

    while let Some(start) = remaining.find("%%\n") {
        let after_open = &remaining[start + 3..];
        let Some(end) = after_open.find("\n%%") else { break };
        let block_content = &after_open[..end];

        if let Ok(block) = parse_single_field_block(block_content) {
            blocks.push(block);
        }

        remaining = &after_open[end + 3..];
        // skip past newline after %%
        if remaining.starts_with('\n') {
            remaining = &remaining[1..];
        }
    }

    blocks
}

fn parse_single_field_block(content: &str) -> Result<FieldBlock> {
    #[derive(Deserialize)]
    struct Raw {
        field: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        format: Option<String>,
        #[serde(default)]
        constraints: Option<String>,
    }
    let raw: Raw = serde_yaml::from_str(content)?;
    Ok(FieldBlock {
        field: raw.field,
        description: raw.description,
        format: raw.format,
        constraints: raw.constraints,
    })
}

fn extract_body(content: &str) -> String {
    // Find the last occurrence of \n%% closing delimiter and take everything after
    let mut last_end = 0;
    let mut remaining = content;
    let mut offset = 0;

    while let Some(start) = remaining.find("%%\n") {
        let after_open = &remaining[start + 3..];
        let Some(end) = after_open.find("\n%%") else { break };
        let block_len = start + 3 + end + 3;
        offset += block_len;
        // skip newline
        let skip = if after_open[end + 3..].starts_with('\n') { 1 } else { 0 };
        offset += skip;
        remaining = &remaining[block_len + skip..];
        last_end = offset;
    }

    content[last_end..].trim_start_matches('\n').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn templates_dir() -> PathBuf {
        // Resolve relative to the crate root
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        PathBuf::from(manifest).join("templates")
    }

    // Build doc names this test "loads_all_fifteen_templates" but lists 16 files in §6.
    // The name is kept for AC traceability; the count is 16 (12 atomic + 4 source types).
    #[test]
    fn loads_all_fifteen_templates() {
        let dir = templates_dir();
        let registry = TemplateRegistry::load(&dir).expect("should load all templates");
        assert_eq!(
            registry.templates.len(),
            16,
            "expected 16 templates (12 atomic + 4 non-atomic), got {}. Found: {:?}",
            registry.templates.len(),
            registry.all_entity_types()
        );
    }

    #[test]
    fn atomic_types_returns_expected_set() {
        let dir = templates_dir();
        let registry = TemplateRegistry::load(&dir).expect("should load");
        let atomic = registry.atomic_types();
        assert_eq!(
            atomic.len(),
            12,
            "expected 12 atomic types, got {}: {:?}",
            atomic.len(),
            atomic
        );
        // Verify all 12 expected output types are present
        // (5 pure_atomic + 2 container + 4 source_bound + outline)
        let expected = [
            "action_item_list", "area", "concept", "context", "event",
            "note", "organization", "outline", "person", "project", "task", "topic",
        ];
        for t in &expected {
            assert!(atomic.contains(t), "missing atomic type: {t}");
        }
    }

    #[test]
    fn source_types_returns_four() {
        let dir = templates_dir();
        let registry = TemplateRegistry::load(&dir).expect("should load");
        let sources = registry.source_types();
        assert_eq!(
            sources.len(),
            4,
            "expected 4 source types, got {}: {:?}",
            sources.len(),
            sources
        );
    }

    #[test]
    fn container_types_have_roster_sections() {
        let dir = templates_dir();
        let registry = TemplateRegistry::load(&dir).expect("should load");
        let org = registry.get("organization").expect("organization template");
        assert!(!org.roster_sections.is_empty(), "organization should have roster_sections");
        let proj = registry.get("project").expect("project template");
        assert!(!proj.roster_sections.is_empty(), "project should have roster_sections");
    }

    #[test]
    fn render_body_substitutes_fields() {
        let dir = templates_dir();
        let registry = TemplateRegistry::load(&dir).expect("should load");
        let t = registry.get("person").expect("person template");
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), "Ian Kitajima".to_string());
        fields.insert("contact_email".to_string(), "ian@example.com".to_string());
        fields.insert("contact_phone".to_string(), "[not mentioned]".to_string());
        fields.insert("summary".to_string(), "CTO of XYZ".to_string());
        let rendered = t.render_body(&fields);
        assert!(rendered.contains("Ian Kitajima"));
        assert!(rendered.contains("ian@example.com"));
    }
}
