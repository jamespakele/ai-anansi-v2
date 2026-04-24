use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use crate::db::{NoteRecord, SourceRecord};
use crate::template::RosterSection;
use crate::vault::Vault;

pub struct EntityRef {
    pub name: String,
    pub entity_type: String,
    pub slug: String,
}

pub struct OutlineSection {
    pub heading: String,
    pub leaves: Vec<OutlineLeaf>,
}

pub struct OutlineLeaf {
    pub address: String,
    pub name: String,
    pub note_id: String,
    pub entity_type: String,
}

/// Write a YAML-safe value. Quotes if it contains `:`, `#`, newlines, tabs, or leading/trailing whitespace.
fn yaml_value(v: &str) -> String {
    let needs_quote = v.contains(':')
        || v.contains('#')
        || v.contains('\n')
        || v.contains('\r')
        || v.contains('\t')
        || v.starts_with(' ')
        || v.ends_with(' ')
        || v.is_empty();
    if needs_quote {
        let escaped = v
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");
        format!("\"{escaped}\"")
    } else {
        v.to_string()
    }
}

/// Write to `.tmp` then rename for atomicity.
fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory: {}", parent.display()))?;
    }
    std::fs::write(&tmp, content)
        .with_context(|| format!("writing tmp file: {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("renaming {} to {}", tmp.display(), path.display()))?;
    Ok(())
}

/// Write an atomic note file with YAML frontmatter then body.
///
/// Frontmatter order: anansi_id, entity_type, name, match_key, then all fields.
pub fn write_atomic_note(
    _vault: &Vault,
    note: &NoteRecord,
    fields: &HashMap<String, String>,
    body: &str,
) -> Result<()> {
    let path = PathBuf::from(&note.file_path);

    // Collect field keys sorted for stability
    let mut field_pairs: Vec<(String, String)> = fields
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    field_pairs.sort_by(|a, b| a.0.cmp(&b.0));

    // We need owned strings for the pairs but emit_frontmatter takes &str,
    // so build the frontmatter string directly.
    let mut fm = String::from("---\n");
    fm.push_str(&format!("anansi_id: {}\n", yaml_value(&note.id)));
    fm.push_str(&format!("entity_type: {}\n", yaml_value(&note.entity_type)));
    fm.push_str(&format!("name: {}\n", yaml_value(&note.name)));
    fm.push_str(&format!("match_key: {}\n", yaml_value(&note.match_key)));
    for (k, v) in &field_pairs {
        fm.push_str(&format!("{}: {}\n", k, yaml_value(v)));
    }
    fm.push_str("---\n");

    let content = format!("{fm}{body}");
    atomic_write(&path, &content)
        .with_context(|| format!("writing atomic note: {}", path.display()))?;
    Ok(())
}

/// Write an outline note file.
pub fn write_outline(
    vault: &Vault,
    source: &SourceRecord,
    outline_note_id: &str,
    outline_match_key: &str,
    title: &str,
    toc_author: &str,
    leaves: &[crate::pipeline::TocLeaf],
) -> Result<PathBuf> {
    let source_slug = slug_from_path(&source.source_path);
    let path = vault.outline_path(&source_slug);

    let now = chrono::Utc::now().to_rfc3339();

    let mut fm = String::from("---\n");
    fm.push_str(&format!("anansi_id: {}\n", yaml_value(outline_note_id)));
    fm.push_str("entity_type: outline\n");
    fm.push_str(&format!("name: {}\n", yaml_value(title)));
    fm.push_str(&format!("match_key: {}\n", yaml_value(outline_match_key)));
    fm.push_str(&format!("source_id: {}\n", yaml_value(&source.id)));
    fm.push_str(&format!("toc_author: {}\n", yaml_value(toc_author)));
    fm.push_str(&format!("toc_generated_at: {}\n", yaml_value(&now)));
    fm.push_str("---\n");

    let mut body = String::new();
    body.push_str("[[-Outline]]\n\n");
    body.push_str(&format!("# {title} — Outline\n\n"));

    // Group leaves by top-level address number (first segment before first dot)
    let mut sections: std::collections::BTreeMap<u32, Vec<&crate::pipeline::TocLeaf>> =
        std::collections::BTreeMap::new();
    for leaf in leaves {
        let top: u32 = leaf
            .address
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        sections.entry(top).or_default().push(leaf);
    }

    for (section_num, sec_leaves) in &sections {
        body.push_str(&format!("## Section {section_num}\n\n"));
        for leaf in sec_leaves {
            let wikilink = vault.wikilink(&leaf.entity_type, &leaf.name);
            body.push_str(&format!("- {} {wikilink}\n", leaf.address));
        }
        body.push('\n');
    }

    let content = format!("{fm}{body}");
    atomic_write(&path, &content)
        .with_context(|| format!("writing outline: {}", path.display()))?;
    Ok(path)
}

fn slug_from_path(source_path: &str) -> String {
    let p = Path::new(source_path);
    p.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Render a roster section by substituting `{field}` in row_format from each row's HashMap.
pub fn render_roster_section(section: &RosterSection, rows: &[HashMap<String, String>]) -> String {
    let mut out = format!("## {}\n\n", section.render_as);
    for row in rows {
        let mut line = section.row_format.clone();
        for (k, v) in row {
            line = line.replace(&format!("{{{k}}}"), v);
        }
        out.push_str(&format!("- {line}\n"));
    }
    out
}

/// Render an entities section as wikilinks.
pub fn render_entities_section(entities: &[EntityRef], vault: &Vault) -> String {
    let mut out = String::from("## Entities\n\n");
    for e in entities {
        let wikilink = vault.wikilink(&e.entity_type, &e.name);
        out.push_str(&format!("- {wikilink}\n"));
    }
    out
}

/// Render the body of a template, substituting `{{field}}` placeholders and unescaping `\{` to `{`.
pub fn render_body(template_body: &str, fields: &HashMap<String, String>) -> String {
    // Replace {{field}} placeholders
    let mut result = String::with_capacity(template_body.len());
    let mut chars = template_body.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            if chars.peek() == Some(&'{') {
                chars.next(); // consume second {
                // collect field name until }}
                let mut field_name = String::new();
                let mut closed = false;
                while let Some(fc) = chars.next() {
                    if fc == '}' {
                        if chars.peek() == Some(&'}') {
                            chars.next(); // consume second }
                            closed = true;
                            break;
                        } else {
                            field_name.push(fc);
                        }
                    } else {
                        field_name.push(fc);
                    }
                }
                if closed {
                    let val = fields.get(&field_name).map(|s| s.as_str()).unwrap_or("");
                    result.push_str(val);
                } else {
                    // Unclosed — emit as-is
                    result.push_str("{{");
                    result.push_str(&field_name);
                }
            } else {
                result.push(c);
            }
        } else if c == '\\' && chars.peek() == Some(&'{') {
            chars.next(); // consume {
            result.push('{');
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_value_quotes_colon() {
        let v = yaml_value("foo: bar");
        assert!(v.starts_with('"'), "should be quoted");
    }

    #[test]
    fn yaml_value_plain_string() {
        let v = yaml_value("hello world");
        assert_eq!(v, "hello world");
    }

    #[test]
    fn render_body_substitutes() {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), "Ian".to_string());
        fields.insert("email".to_string(), "ian@example.com".to_string());
        let body = "# {{name}}\nEmail: {{email}}\n";
        let result = render_body(body, &fields);
        assert_eq!(result, "# Ian\nEmail: ian@example.com\n");
    }

    #[test]
    fn render_body_unescapes_backslash_brace() {
        let fields = HashMap::new();
        let body = r"Use \{literal\} braces";
        let result = render_body(body, &fields);
        assert!(result.contains('{'), "should contain literal brace");
    }

    #[test]
    fn render_roster_section_substitutes_fields() {
        let section = RosterSection {
            source_field: "members".to_string(),
            render_as: "Members".to_string(),
            row_format: "{name} ({role})".to_string(),
            dedupe_by: vec!["name".to_string()],
        };
        let mut row = HashMap::new();
        row.insert("name".to_string(), "Alice".to_string());
        row.insert("role".to_string(), "Lead".to_string());
        let result = render_roster_section(&section, &[row]);
        assert!(result.contains("Alice (Lead)"), "row should be rendered");
        assert!(result.contains("## Members"), "heading should appear");
    }
}
