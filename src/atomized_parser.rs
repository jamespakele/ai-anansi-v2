use anyhow::{anyhow, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedEdge {
    pub edge_type: String,
    pub target_mk: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedBlock {
    pub address: String,
    pub title: String,
    pub entity_type: String,
    pub lede: String,
    pub why: Option<String>,
    pub content: Option<String>,
    pub edges: Vec<ParsedEdge>,
}

#[derive(Debug, Clone)]
pub struct AtomizedFile {
    pub source_title: String,
    pub date: String,
    pub variant: String,
    pub blocks: Vec<ParsedBlock>,
    pub skipped_count: usize,
    pub concepts: Vec<String>,
    pub raw_toc: String,
}

pub fn parse_atomized_file(text: &str) -> Result<AtomizedFile> {
    // Step 1: find and parse the file header
    let (source_title, date, variant, after_header) = parse_header(text)?;

    // Step 2: find and remove the concepts footer line
    let (concepts, body) = extract_concepts(&after_header);

    // Step 3: split on standalone --- lines, owning the segments
    let owned_segments: Vec<String> = body
        .split('\n')
        .collect::<Vec<_>>()
        .split(|line: &&str| line.trim() == "---")
        .map(|chunk| chunk.join("\n"))
        .filter(|seg| !seg.trim().is_empty())
        .collect();

    let mut blocks = Vec::new();
    let mut skipped_count = 0usize;

    for seg in &owned_segments {
        match parse_block(seg) {
            Ok(block) => blocks.push(block),
            Err(_) => skipped_count += 1,
        }
    }

    // Step 6: build raw_toc from block headers in address order
    let raw_toc = blocks
        .iter()
        .map(|b| format!("{} {} [{}]", b.address, b.title, b.entity_type))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(AtomizedFile {
        source_title,
        date,
        variant,
        blocks,
        skipped_count,
        concepts,
        raw_toc,
    })
}

fn parse_header(text: &str) -> Result<(String, String, String, String)> {
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with("<!--") || !trimmed.ends_with("-->") {
            continue;
        }
        let inner = trimmed
            .trim_start_matches("<!--")
            .trim_end_matches("-->")
            .trim();

        let variant;
        let payload;

        if let Some(rest) = inner.strip_prefix("anansi-atomize:") {
            variant = "toc".to_string();
            payload = rest.trim();
        } else if let Some(rest) = inner.strip_prefix("smart-brevity atomization:") {
            variant = "sb".to_string();
            payload = rest.trim();
        } else {
            continue;
        }

        let parts: Vec<&str> = payload.splitn(3, '|').collect();
        if parts.len() < 3 {
            return Err(anyhow!(
                "File header must have exactly two '|' separators: {trimmed}"
            ));
        }
        let source_title = parts[0].trim().to_string();
        let date = parts[2].trim().to_string();

        // Everything after the header line
        let after_header = text
            .lines()
            .skip(i + 1)
            .collect::<Vec<_>>()
            .join("\n");

        return Ok((source_title, date, variant, after_header));
    }
    Err(anyhow!(
        "No recognised file header found (expected <!-- anansi-atomize: ... --> or <!-- smart-brevity atomization: ... -->)"
    ))
}

fn extract_concepts(text: &str) -> (Vec<String>, String) {
    let mut concepts = Vec::new();
    let mut body_lines = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<!--") && trimmed.ends_with("-->") {
            let inner = trimmed
                .trim_start_matches("<!--")
                .trim_end_matches("-->")
                .trim();
            if let Some(rest) = inner.strip_prefix("concepts:") {
                concepts = rest
                    .split_whitespace()
                    .filter(|t| t.starts_with('#'))
                    .map(|t| t.to_string())
                    .collect();
                continue; // don't include this line in the body
            }
        }
        body_lines.push(line);
    }

    (concepts, body_lines.join("\n"))
}

fn parse_block(segment: &str) -> Result<ParsedBlock> {
    let lines: Vec<&str> = segment.lines().collect();

    // Find first non-empty line — must match the block header regex
    let header_idx = lines
        .iter()
        .position(|l| !l.trim().is_empty())
        .ok_or_else(|| anyhow!("empty segment"))?;

    let header_line = lines[header_idx].trim();
    let (address, title, entity_type) = parse_block_header(header_line)?;

    // Find ### Edges boundary
    let edges_idx = lines
        .iter()
        .skip(header_idx + 1)
        .position(|l| is_edges_heading(l))
        .map(|rel| rel + header_idx + 1);

    let body_lines: &[&str];
    let edge_lines: &[&str];

    if let Some(ei) = edges_idx {
        body_lines = &lines[header_idx + 1..ei];
        edge_lines = &lines[ei + 1..];
    } else {
        body_lines = &lines[header_idx + 1..];
        edge_lines = &[];
    }

    // Find lede: first non-empty line in body_lines
    let lede_idx = body_lines
        .iter()
        .position(|l| !l.trim().is_empty())
        .ok_or_else(|| anyhow!("block has no lede: {header_line}"))?;

    let lede_candidate = body_lines[lede_idx].trim();

    // Guard: if lede looks like a Why line, the block is malformed (missing lede)
    if lede_candidate.starts_with("**Why it matters:**") {
        return Err(anyhow!(
            "block missing lede — first body line is a Why line: {header_line}"
        ));
    }

    let lede = lede_candidate.to_string();

    // Scan body_lines for a Why line (after the lede)
    let why_idx = body_lines
        .iter()
        .skip(lede_idx + 1)
        .position(|l| l.trim().starts_with("**Why it matters:**"))
        .map(|rel| rel + lede_idx + 1);

    let why: Option<String>;
    let content_lines: &[&str];

    if let Some(wi) = why_idx {
        let why_text = body_lines[wi]
            .trim()
            .trim_start_matches("**Why it matters:**")
            .trim()
            .to_string();
        why = Some(why_text);
        content_lines = &body_lines[wi + 1..];
    } else {
        why = None;
        content_lines = &body_lines[lede_idx + 1..];
    }

    let content_str = content_lines.join("\n").trim().to_string();
    let content = if content_str.is_empty() {
        None
    } else {
        Some(content_str)
    };

    // Parse edge lines
    let edges = parse_edge_lines(edge_lines);

    Ok(ParsedBlock {
        address,
        title,
        entity_type,
        lede,
        why,
        content,
        edges,
    })
}

fn parse_block_header(line: &str) -> Result<(String, String, String)> {
    // ### {address} {title} [{entity_type}]
    let without_hashes = line
        .trim_start_matches('#')
        .trim();

    // Find the [...] at the end
    let bracket_start = without_hashes
        .rfind('[')
        .ok_or_else(|| anyhow!("no entity_type bracket: {line}"))?;
    let bracket_end = without_hashes
        .rfind(']')
        .ok_or_else(|| anyhow!("no closing bracket: {line}"))?;

    if bracket_end <= bracket_start {
        return Err(anyhow!("malformed brackets: {line}"));
    }

    let entity_type = without_hashes[bracket_start + 1..bracket_end].trim().to_string();

    // Validate entity_type matches [A-Za-z][A-Za-z0-9_-]*
    if entity_type.is_empty()
        || !entity_type.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false)
        || !entity_type.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(anyhow!("invalid entity_type '{entity_type}': {line}"));
    }

    let before_bracket = without_hashes[..bracket_start].trim();

    // Split into address and title: address is the leading decimal number
    let space_idx = before_bracket
        .find(' ')
        .ok_or_else(|| anyhow!("no space between address and title: {line}"))?;

    let address = before_bracket[..space_idx].trim().to_string();
    let title = before_bracket[space_idx..].trim().to_string();

    // Validate address starts with a digit
    if !address.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return Err(anyhow!("address does not start with digit: {line}"));
    }

    // Validate it's a proper decimal address (digits and dots only)
    if !address.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Err(anyhow!("invalid address format: {line}"));
    }

    if title.is_empty() {
        return Err(anyhow!("empty title: {line}"));
    }

    // Must have ### prefix in original
    if !line.trim_start().starts_with("###") {
        return Err(anyhow!("not a block header (missing ###): {line}"));
    }

    Ok((address, title, entity_type))
}

fn is_edges_heading(line: &str) -> bool {
    let t = line.trim();
    // Must be exactly "### Edges" (possibly with leading/trailing whitespace)
    t.starts_with("###") && t.trim_start_matches('#').trim() == "Edges"
}

fn parse_edge_lines(lines: &[&str]) -> Vec<ParsedEdge> {
    let mut edges = Vec::new();
    for line in lines {
        let t = line.trim();
        if !t.starts_with('-') {
            continue;
        }
        let rest = t.trim_start_matches('-').trim();
        // Format: edge_type: entity_type:slug
        let colon_idx = match rest.find(':') {
            Some(i) => i,
            None => continue,
        };
        let edge_type = rest[..colon_idx].trim();
        let target_mk = rest[colon_idx + 1..].trim();

        // Validate edge_type: [a-z_]+
        if edge_type.is_empty()
            || !edge_type.chars().all(|c| c.is_ascii_lowercase() || c == '_')
        {
            continue;
        }

        // Validate target_mk: entity_type:slug where both parts are lowercase-alphanumeric-hyphen
        let parts: Vec<&str> = target_mk.splitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }
        let et = parts[0];
        let slug = parts[1];

        if et.is_empty()
            || !et.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false)
            || !et.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            continue;
        }
        if slug.is_empty()
            || !slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            continue;
        }

        edges.push(ParsedEdge {
            edge_type: edge_type.to_string(),
            target_mk: target_mk.to_string(),
        });
    }
    edges
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const ATOMIZE_HEADER: &str =
        "<!-- anansi-atomize: Test Source | 3 toc-blocks | 2026-04-30 -->";
    const SB_HEADER: &str =
        "<!-- smart-brevity atomization: Test Source | 3 blocks | 2026-04-30 -->";

    fn make_file(header: &str, blocks: &str, footer: &str) -> String {
        format!("{header}\n{blocks}\n{footer}")
    }

    fn simple_block_a() -> &'static str {
        "### 3.1 Introduction [book-chapter]\nThe book begins here.\n\n**Why it matters:** Sets context for everything.\n\nThis is the body content.\n\n---"
    }

    fn simple_block_b() -> &'static str {
        "### 4.1 Ian Kitajima [person]\nHawaii DOE innovation lead.\n\n- Contact: ian@doe.hawaii.gov\n- Role: director\n\n---"
    }

    #[test]
    fn parse_anansi_atomize_header() {
        let text = make_file(ATOMIZE_HEADER, simple_block_a(), "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        assert_eq!(result.source_title, "Test Source");
        assert_eq!(result.date, "2026-04-30");
    }

    #[test]
    fn parse_smart_brevity_header() {
        let text = make_file(SB_HEADER, simple_block_b(), "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        assert_eq!(result.source_title, "Test Source");
        assert_eq!(result.date, "2026-04-30");
    }

    #[test]
    fn variant_toc() {
        let text = make_file(ATOMIZE_HEADER, simple_block_a(), "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        assert_eq!(result.variant, "toc");
    }

    #[test]
    fn variant_sb() {
        let text = make_file(SB_HEADER, simple_block_b(), "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        assert_eq!(result.variant, "sb");
    }

    #[test]
    fn parse_shape_a_block() {
        // Shape A: has sub-headings and why
        let text = make_file(ATOMIZE_HEADER, simple_block_a(), "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        assert_eq!(result.blocks.len(), 1);
        let block = &result.blocks[0];
        assert_eq!(block.address, "3.1");
        assert_eq!(block.lede, "The book begins here.");
        assert_eq!(block.why.as_deref(), Some("Sets context for everything."));
        assert_eq!(block.content.as_deref(), Some("This is the body content."));
    }

    #[test]
    fn parse_shape_b_block() {
        // Shape B: bullet list content, no why
        let text = make_file(ATOMIZE_HEADER, simple_block_b(), "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        assert_eq!(result.blocks.len(), 1);
        let block = &result.blocks[0];
        assert_eq!(block.lede, "Hawaii DOE innovation lead.");
        assert!(block.why.is_none());
        assert!(block.content.is_some());
        assert!(block.content.as_deref().unwrap().contains("ian@doe.hawaii.gov"));
    }

    #[test]
    fn parse_omits_why() {
        let seg = "### 1.1 Project Alpha [project]\nActive project.\n\nSome bullets.\n\n---";
        let text = make_file(ATOMIZE_HEADER, seg, "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        assert!(result.blocks[0].why.is_none());
    }

    #[test]
    fn parse_empty_content() {
        let seg = "### 2.1 Area One [area]\nOngoing responsibility.\n\n**Why it matters:** Core area.\n\n---";
        let text = make_file(ATOMIZE_HEADER, seg, "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        let block = &result.blocks[0];
        assert_eq!(block.why.as_deref(), Some("Core area."));
        assert!(block.content.is_none(), "expected None content, got {:?}", block.content);
    }

    #[test]
    fn parse_concepts() {
        let seg = "### 1.1 Project Alpha [project]\nActive.\n\n---";
        let footer = "<!-- concepts: #closed-learning-loop #anansi #knowledge-management -->";
        let text = make_file(ATOMIZE_HEADER, seg, footer);
        let result = parse_atomized_file(&text).unwrap();
        assert_eq!(
            result.concepts,
            vec!["#closed-learning-loop", "#anansi", "#knowledge-management"]
        );
    }

    #[test]
    fn parse_skips_malformed() {
        let bad = "Not a block at all.\n\n---";
        let good = "### 1.1 Good Block [project]\nGood lede.\n\n---";
        let text = make_file(ATOMIZE_HEADER, &format!("{bad}\n{good}"), "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        assert_eq!(result.blocks.len(), 1, "good block parsed");
        assert_eq!(result.skipped_count, 1, "bad block counted as skipped");
    }

    #[test]
    fn raw_toc_format() {
        let seg = "### 1.1 Project Alpha [project]\nActive.\n\n---\n### 4.1 Ian K [person]\nPerson.\n\n---";
        let text = make_file(ATOMIZE_HEADER, seg, "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        let lines: Vec<&str> = result.raw_toc.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "1.1 Project Alpha [project]");
        assert_eq!(lines[1], "4.1 Ian K [person]");
    }

    #[test]
    fn parse_edges_section() {
        let seg = "### 1.1 ConCon [project]\nA civic tech project.\n\n**Why it matters:** Community impact.\n\nSome content.\n\n### Edges\n- owner: person:richmond\n- references: organization:hipa\n\n---";
        let text = make_file(ATOMIZE_HEADER, seg, "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        let block = &result.blocks[0];
        assert_eq!(block.edges.len(), 2);
        assert_eq!(block.edges[0].edge_type, "owner");
        assert_eq!(block.edges[0].target_mk, "person:richmond");
        assert_eq!(block.edges[1].edge_type, "references");
        assert_eq!(block.edges[1].target_mk, "organization:hipa");
        // Edges section must NOT appear in content
        let content = block.content.as_deref().unwrap_or("");
        assert!(!content.contains("### Edges"), "Edges section leaked into content");
        assert!(!content.contains("person:richmond"), "Edge line leaked into content");
    }

    #[test]
    fn parse_no_edges_section() {
        let text = make_file(ATOMIZE_HEADER, simple_block_b(), "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        let block = &result.blocks[0];
        assert!(block.edges.is_empty());
        // Content should be unaffected
        let content = block.content.as_deref().unwrap_or("");
        assert!(content.contains("ian@doe.hawaii.gov"));
    }

    #[test]
    fn parse_edges_content_boundary() {
        let seg = "### 1.1 ConCon [project]\nThe lede.\n\n**Why it matters:** Reason.\n\nThis is real content.\nMore content.\n\n### Edges\n- owner: person:richmond\n\n---";
        let text = make_file(ATOMIZE_HEADER, seg, "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        let block = &result.blocks[0];
        let content = block.content.as_deref().unwrap_or("");
        assert!(content.contains("This is real content."));
        assert!(content.contains("More content."));
        assert!(!content.contains("### Edges"));
        assert!(!content.contains("person:richmond"));
    }

    #[test]
    fn header_requires_two_pipes() {
        // Only one pipe — missing the block-count field
        let text = "<!-- anansi-atomize: Test Source | 2026-04-30 -->\n### 1.1 Block [project]\nLede.\n---\n<!-- concepts: -->";
        let result = parse_atomized_file(text);
        assert!(result.is_err(), "should fail with only one pipe in header");
    }

    #[test]
    fn lede_as_why_is_malformed() {
        // Block where first body line is a Why line (missing lede)
        let seg = "### 1.1 Block [project]\n**Why it matters:** Some reason.\n\n---";
        let text = make_file(ATOMIZE_HEADER, seg, "<!-- concepts: -->");
        let result = parse_atomized_file(&text).unwrap();
        // Block should be skipped (malformed)
        assert_eq!(result.blocks.len(), 0);
        assert_eq!(result.skipped_count, 1);
    }
}
