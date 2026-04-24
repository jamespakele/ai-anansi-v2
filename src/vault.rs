use std::path::{Path, PathBuf};

/// Slug generation per spec §13 — identical algorithm to match_key normalization
/// (lowercase, non-alphanumeric → space, collapse whitespace, join with hyphens).
fn slug_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

pub struct Vault {
    pub root: PathBuf,
    pub web: PathBuf,
}

impl Vault {
    pub fn new(root: PathBuf, web_dir: &Path) -> Self {
        let web = if web_dir.is_absolute() {
            web_dir.to_path_buf()
        } else {
            root.join(web_dir)
        };
        Self { root, web }
    }

    pub fn source_path(&self, source_slug: &str) -> PathBuf {
        self.root.join(format!("{source_slug}.md"))
    }

    pub fn outline_path(&self, source_slug: &str) -> PathBuf {
        self.web.join(format!("{source_slug}.outline.md"))
    }

    pub fn atomic_note_path(&self, entity_type: &str, name: &str) -> PathBuf {
        let s = slug_name(name);
        let ext = if entity_type.is_empty() { "note" } else { entity_type };
        self.web.join(format!("{s}.{ext}.md"))
    }

    pub fn source_bound_path(&self, toc_address: &str, name: &str, source_slug: &str) -> PathBuf {
        let addr = toc_address.replace('.', "-");
        let s = slug_name(name);
        self.web.join(format!("{addr}-{s}-{source_slug}.md"))
    }

    pub fn wikilink(&self, entity_type: &str, name: &str) -> String {
        let s = slug_name(name);
        let ext = if entity_type.is_empty() { "note" } else { entity_type };
        format!("[[{s}.{ext}|{name}]]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault() -> Vault {
        Vault::new(PathBuf::from("/anansi"), Path::new("web"))
    }

    #[test]
    fn path_conventions() {
        let v = vault();

        // source path
        assert_eq!(
            v.source_path("digital-futures-workshop-2026-01-30"),
            PathBuf::from("/anansi/digital-futures-workshop-2026-01-30.md")
        );

        // outline path
        assert_eq!(
            v.outline_path("digital-futures-workshop-2026-01-30"),
            PathBuf::from("/anansi/web/digital-futures-workshop-2026-01-30.outline.md")
        );

        // pure-atomic: person gets .person.md
        assert_eq!(
            v.atomic_note_path("person", "Ian Kitajima"),
            PathBuf::from("/anansi/web/ian-kitajima.person.md")
        );

        // pure-atomic: organization gets .organization.md
        assert_eq!(
            v.atomic_note_path("organization", "PICHTR"),
            PathBuf::from("/anansi/web/pichtr.organization.md")
        );

        // pure-atomic: concept gets .concept.md
        assert_eq!(
            v.atomic_note_path("concept", "Sovereign AI"),
            PathBuf::from("/anansi/web/sovereign-ai.concept.md")
        );

        // note type gets .note.md
        assert_eq!(
            v.atomic_note_path("note", "some note"),
            PathBuf::from("/anansi/web/some-note.note.md")
        );

        // empty entity_type falls back to .note.md
        assert_eq!(
            v.atomic_note_path("", "fallback note"),
            PathBuf::from("/anansi/web/fallback-note.note.md")
        );

        // source-bound: dots in address become hyphens
        assert_eq!(
            v.source_bound_path("3.2", "Sovereign AI discussion", "dfw-2026-01"),
            PathBuf::from("/anansi/web/3-2-sovereign-ai-discussion-dfw-2026-01.md")
        );
    }

    #[test]
    fn wikilink_formats() {
        let v = vault();

        // person → .person extension
        assert_eq!(v.wikilink("person", "Ian Kitajima"), "[[ian-kitajima.person|Ian Kitajima]]");

        // concept → .concept extension
        assert_eq!(v.wikilink("concept", "Sovereign AI"), "[[sovereign-ai.concept|Sovereign AI]]");

        // note → .note extension
        assert_eq!(v.wikilink("note", "some note"), "[[some-note.note|some note]]");

        // empty → .note fallback
        assert_eq!(v.wikilink("", "fallback"), "[[fallback.note|fallback]]");
    }
}
