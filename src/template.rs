use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateClass {
    Identity,
    ContentUnit,
    Source,
    Utility,
}

impl<'de> Deserialize<'de> for TemplateClass {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        match s.as_str() {
            "identity" => Ok(TemplateClass::Identity),
            "content_unit" => Ok(TemplateClass::ContentUnit),
            "source" => Ok(TemplateClass::Source),
            "utility" => Ok(TemplateClass::Utility),
            other => Err(serde::de::Error::custom(format!("unknown template_class: {other}"))),
        }
    }
}

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
    template_class: TemplateClass,
    #[serde(default)]
    atomic: bool,
    merge_strategy: MergeStrategy,
    #[serde(default)]
    template_version: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    floor_prompt: Option<String>,
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
    pub template_class: TemplateClass,
    pub atomic: bool,
    pub merge_strategy: MergeStrategy,
    pub template_version: String,
    pub description: String,
    pub floor_prompt: Option<String>,
    pub source_family: Option<String>,
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

            let mut template = parse_template(&content)
                .with_context(|| format!("parsing template: {}", path.display()))?;

            // Parse source_family from filename prefix (e.g. "meeting-topic-discussion" → "meeting")
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            template.source_family = if stem.starts_with("identity-") || !stem.contains('-') {
                None
            } else {
                stem.splitn(2, '-').next().map(|s| s.to_string())
            };

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

    /// Returns entity types relevant to the given source_type:
    /// identity + utility classes, plus any content_unit/source types
    /// whose source_family matches the source type's family.
    pub fn types_for_source(&self, source_type: &str) -> Vec<&str> {
        let family = Self::source_family_of(source_type);
        let mut types: Vec<&str> = self.templates.values()
            .filter(|t| {
                matches!(t.template_class, TemplateClass::Identity | TemplateClass::Utility)
                    || (!family.is_empty()
                        && t.source_family.as_deref() == Some(family))
            })
            .map(|t| t.entity_type.as_str())
            .collect();
        types.sort();
        types
    }

    /// Returns floor_prompt strings from content_unit templates whose
    /// source_family matches the source type's family.
    pub fn floor_prompts_for_source(&self, source_type: &str) -> Vec<&str> {
        let family = Self::source_family_of(source_type);
        if family.is_empty() {
            return Vec::new();
        }
        self.templates.values()
            .filter(|t| {
                t.template_class == TemplateClass::ContentUnit
                    && t.source_family.as_deref() == Some(family)
            })
            .filter_map(|t| t.floor_prompt.as_deref())
            .collect()
    }

    fn source_family_of(source_type: &str) -> &str {
        match source_type {
            "meeting_summary" => "meeting",
            "research_paper"  => "research",
            "email_thread"    => "email",
            "youtube_video"   => "youtube",
            _                 => "",
        }
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
        template_class: fm.template_class,
        atomic: fm.atomic,
        merge_strategy: fm.merge_strategy,
        template_version: fm.template_version,
        description: fm.description,
        floor_prompt: fm.floor_prompt,
        source_family: None, // set by load() from filename
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

    #[test]
    fn loads_all_templates() {
        let dir = templates_dir();
        let registry = TemplateRegistry::load(&dir).expect("should load all templates");
        assert!(
            registry.templates.len() >= 21,
            "expected at least 21 templates, got {}. Found: {:?}",
            registry.templates.len(),
            registry.all_entity_types()
        );
    }

    #[test]
    fn atomic_types_returns_expected_set() {
        let dir = templates_dir();
        let registry = TemplateRegistry::load(&dir).expect("should load");
        let atomic = registry.atomic_types();
        assert!(
            atomic.len() >= 16,
            "expected at least 16 atomic types, got {}: {:?}",
            atomic.len(),
            atomic
        );
        let expected = [
            "action_item_list", "area", "article_section", "context",
            "email_exchange", "event", "note", "organization", "outline",
            "person", "project", "task", "topic", "topic_discussion", "youtube_chapter",
        ];
        for t in &expected {
            assert!(atomic.contains(t), "missing atomic type: {t}");
        }
    }

    #[test]
    fn source_types_returns_five() {
        let dir = templates_dir();
        let registry = TemplateRegistry::load(&dir).expect("should load");
        let sources = registry.source_types();
        assert!(
            sources.len() >= 5,
            "expected at least 5 source types, got {}: {:?}",
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
