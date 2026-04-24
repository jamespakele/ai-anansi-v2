use anyhow::{anyhow, Result};
use crate::rules::RuleRegistry;
use crate::template::TemplateRegistry;

static PASS1_TEMPLATE: &str = include_str!("../prompts/pass-1-toc-extraction.md");
static PASS3_TEMPLATE: &str = include_str!("../prompts/pass-3-node-expansion.md");
static PASS4_TEMPLATE: &str = include_str!("../prompts/pass-4-relationship-extraction.md");

const RELATIONSHIP_TYPES: &[&str] = &[
    "member_of", "belongs_to", "contains", "related_to", "discusses", "involves",
    "produced", "assigned_to", "part_of", "depends_on", "references", "addresses",
    "led_by", "funded_by", "supports", "contradicts", "extends", "precedes",
    "follows", "has_member",
];

/// Simple string replacement for both `{RULES:Name}` and `{NAME}` patterns.
fn inject(template: &str, key: &str, value: &str) -> String {
    template.replace(&format!("{{{key}}}"), value)
}

fn inject_rule(template: &str, rule_name: &str, rules: &RuleRegistry) -> Result<String> {
    let value = rules
        .get(rule_name)
        .ok_or_else(|| anyhow!("rule not found: {rule_name}"))?;
    Ok(inject(template, &format!("RULES:{rule_name}"), value))
}

fn render_entity_types(templates: &TemplateRegistry) -> String {
    templates
        .atomic_types()
        .iter()
        .map(|t| format!("- [{t}]"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_template_fields(entity_type: &str, templates: &TemplateRegistry) -> String {
    let Some(tmpl) = templates.get(entity_type) else {
        return String::new();
    };
    tmpl.field_blocks
        .iter()
        .map(|fb| {
            let mut line = format!("**{}**: {}", fb.field, fb.description);
            if let Some(fmt) = &fb.format {
                line.push_str(&format!(" (format: {fmt})"));
            }
            if let Some(con) = &fb.constraints {
                line.push_str(&format!(" [constraint: {con}]"));
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn build_pass1(
    rules: &RuleRegistry,
    templates: &TemplateRegistry,
    source: &str,
) -> Result<String> {
    let mut prompt = PASS1_TEMPLATE.to_string();
    prompt = inject_rule(&prompt, "Atomicity", rules)?;
    prompt = inject_rule(&prompt, "Downstream-Flow", rules)?;
    let entity_types = render_entity_types(templates);
    prompt = inject(&prompt, "ENTITY_TYPES", &entity_types);
    prompt = inject(&prompt, "SOURCE", source);
    Ok(prompt)
}

pub struct Pass3Params<'a> {
    pub entity_type: &'a str,
    pub entity_name: &'a str,
    pub toc_address: &'a str,
    pub source_hint: &'a str,
    pub leaf_hint: &'a str,
    pub context_at: &'a [String],
    pub source: &'a str,
}

pub fn build_pass3(
    rules: &RuleRegistry,
    templates: &TemplateRegistry,
    params: Pass3Params<'_>,
) -> Result<String> {
    let mut prompt = PASS3_TEMPLATE.to_string();
    prompt = inject_rule(&prompt, "Atomicity", rules)?;
    prompt = inject(&prompt, "ENTITY_TYPE", params.entity_type);
    prompt = inject(&prompt, "ENTITY_NAME", params.entity_name);
    prompt = inject(&prompt, "TOC_ADDRESS", params.toc_address);
    prompt = inject(&prompt, "SOURCE_HINT", params.source_hint);
    prompt = inject(&prompt, "LEAF_HINT", params.leaf_hint);

    let template_fields = render_template_fields(params.entity_type, templates);
    prompt = inject(&prompt, "TEMPLATE_FIELDS", &template_fields);

    let context_at_str = if params.context_at.is_empty() {
        String::new()
    } else {
        format!(
            "Content specific to this source about this entity belongs in context nodes at: {}. Do not duplicate that content here.",
            params.context_at.join(", ")
        )
    };
    prompt = inject(&prompt, "CONTEXT_AT", &context_at_str);
    prompt = inject(&prompt, "SOURCE", params.source);
    Ok(prompt)
}

pub struct Pass4Params<'a> {
    /// Each node as "match_key | entity_type | name | summary_1"
    pub nodes: &'a str,
    pub toc: &'a str,
    pub implicit_edges: &'a str,
}

pub fn build_pass4(params: Pass4Params<'_>) -> String {
    let rel_types = RELATIONSHIP_TYPES.join(", ");
    let mut prompt = PASS4_TEMPLATE.to_string();
    prompt = inject(&prompt, "NODES", params.nodes);
    prompt = inject(&prompt, "TOC", params.toc);
    prompt = inject(&prompt, "IMPLICIT_EDGES", params.implicit_edges);
    prompt = inject(&prompt, "RELATIONSHIP_TYPES", &rel_types);
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn manifest_dir() -> PathBuf {
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set"))
    }

    fn load_templates() -> TemplateRegistry {
        TemplateRegistry::load(&manifest_dir().join("templates")).expect("load templates")
    }

    fn load_rules() -> RuleRegistry {
        RuleRegistry::load(&manifest_dir().join("%Rules")).expect("load rules")
    }

    #[test]
    fn build_pass1_contains_rules_and_entity_types() {
        let templates = load_templates();
        let rules = load_rules();
        let prompt = build_pass1(&rules, &templates, "This is the source content.").unwrap();

        // Rules injected
        assert!(!prompt.contains("{RULES:Atomicity}"), "Atomicity placeholder not replaced");
        assert!(!prompt.contains("{RULES:Downstream-Flow}"), "Downstream-Flow placeholder not replaced");
        // Entity types injected
        assert!(!prompt.contains("{ENTITY_TYPES}"), "ENTITY_TYPES placeholder not replaced");
        assert!(prompt.contains("- [person]"), "person entity type should be listed");
        assert!(prompt.contains("- [concept]"), "concept entity type should be listed");
        // Source injected
        assert!(prompt.contains("This is the source content."), "source content should appear");
        assert!(!prompt.contains("{SOURCE}"), "SOURCE placeholder not replaced");
    }

    #[test]
    fn build_pass3_all_placeholders_replaced() {
        let templates = load_templates();
        let rules = load_rules();
        let params = Pass3Params {
            entity_type: "person",
            entity_name: "Ian Kitajima",
            toc_address: "1.1",
            source_hint: "Check attendee list",
            leaf_hint: "Research director at PICHTR",
            context_at: &["1.3".to_string()],
            source: "Source body text here.",
        };
        let prompt = build_pass3(&rules, &templates, params).unwrap();

        assert!(!prompt.contains("{RULES:Atomicity}"), "Atomicity placeholder not replaced");
        assert!(!prompt.contains("{ENTITY_TYPE}"), "ENTITY_TYPE placeholder not replaced");
        assert!(!prompt.contains("{ENTITY_NAME}"), "ENTITY_NAME placeholder not replaced");
        assert!(!prompt.contains("{TOC_ADDRESS}"), "TOC_ADDRESS placeholder not replaced");
        assert!(!prompt.contains("{SOURCE_HINT}"), "SOURCE_HINT placeholder not replaced");
        assert!(!prompt.contains("{LEAF_HINT}"), "LEAF_HINT placeholder not replaced");
        assert!(!prompt.contains("{TEMPLATE_FIELDS}"), "TEMPLATE_FIELDS placeholder not replaced");
        assert!(!prompt.contains("{CONTEXT_AT}"), "CONTEXT_AT placeholder not replaced");
        assert!(!prompt.contains("{SOURCE}"), "SOURCE placeholder not replaced");

        assert!(prompt.contains("Ian Kitajima"), "entity name should appear");
        assert!(prompt.contains("person"), "entity type should appear");
        assert!(prompt.contains("1.3"), "context_at should appear");
        assert!(prompt.contains("Source body text here."), "source should appear");
        // Template fields from person template
        assert!(prompt.contains("contact_email") || prompt.contains("name"), "person fields should appear");
    }

    #[test]
    fn build_pass4_all_placeholders_replaced() {
        let params = Pass4Params {
            nodes: "person:ian-kitajima | person | Ian Kitajima | Research director at PICHTR",
            toc: "1.1 Ian Kitajima [person]",
            implicit_edges: "outline:test contains person:ian-kitajima",
        };
        let prompt = build_pass4(params);

        assert!(!prompt.contains("{NODES}"), "NODES placeholder not replaced");
        assert!(!prompt.contains("{TOC}"), "TOC placeholder not replaced");
        assert!(!prompt.contains("{IMPLICIT_EDGES}"), "IMPLICIT_EDGES placeholder not replaced");
        assert!(!prompt.contains("{RELATIONSHIP_TYPES}"), "RELATIONSHIP_TYPES placeholder not replaced");

        assert!(prompt.contains("Ian Kitajima"), "node content should appear");
        assert!(prompt.contains("member_of"), "relationship types should appear");
        assert!(prompt.contains("led_by"), "relationship types should appear");
    }

    #[test]
    fn build_pass3_empty_context_at() {
        let templates = load_templates();
        let rules = load_rules();
        let params = Pass3Params {
            entity_type: "context",
            entity_name: "Workshop Discussion",
            toc_address: "1.3",
            source_hint: "",
            leaf_hint: "main discussion section",
            context_at: &[],
            source: "Discussion content.",
        };
        let prompt = build_pass3(&rules, &templates, params).unwrap();
        // CONTEXT_AT should be replaced with empty string
        assert!(!prompt.contains("{CONTEXT_AT}"), "CONTEXT_AT placeholder not replaced");
        // Should not have the context routing sentence
        assert!(!prompt.contains("context nodes at:"), "empty context_at should not emit routing text");
    }
}
