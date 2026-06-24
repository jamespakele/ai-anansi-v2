---
title: 'Anansi v2 — Build 21: realign lint prompt to atomization vocabulary'
type: 'chore'
created: '2026-06-24'
status: 'done'
baseline_commit: 'ed5647d'
context:
  - _bmad-output/implementation-artifacts/spec-build-14-semantic-lint.md
---

## Intent

**Problem:** Verification of the llm-wiki confirmed it's a faithful projection of Anansi's atomized notes + edges everywhere EXCEPT the semantic-lint prompt (`prompts/lint.txt`), whose `under_linked`/`gaps` wording leans on the Karpathy "concept page to create by context" / free-form "relates-to" linking model — misaligned with Anansi, where concepts ARE typed atomic notes and links ARE typed edges (part_of/contains/relates) created by the atomization pipeline or `anansi_relate`. The lint is advisory-only (cannot mutate the graph), so this is framing, not behavior — but it should speak Anansi's language.

**Approach:** Rewrite `prompts/lint.txt` prose to Anansi's atomization vocabulary while keeping the EXACT JSON contract (`contradictions`/`stale`/`under_linked`/`gaps` with the same sub-fields `with`/`detail`/`suggest`/`concept`) so `lint.rs`'s parser and tests are unaffected. Reframe `under_linked` as a missing **typed edge** to an existing atomized note, and `gaps` as a referenced entity that **should be atomized into its own typed note**. Reference `entity_type`/`edge_type` explicitly. Prompt-only; no code change.

## Boundaries & Constraints

**Always:** Keep the JSON keys + sub-field names byte-identical (`lint.rs` `RawFindings`/`Contradiction`/`Detail`/`UnderLinked`/`Gap` deserialize them). Keep the four checks. Keep `{{NOTE}}`/`{{NEIGHBORS}}` placeholders. Stay advisory (report-only).

**Never:** Do not change the JSON schema (would break the parser). Do not touch `lint.rs` or any other code. Do not add or remove a check.

## Code Map

- `prompts/lint.txt` -- rewrite the framing/rules in Anansi atomization terms (typed atomic notes + typed edges; concepts = atomized notes; links = edges). Keep the JSON object shape and placeholders exactly.

## Tasks & Acceptance

**Execution:**
- [x] `prompts/lint.txt` -- realign vocabulary; preserve the exact JSON contract + placeholders.

**Acceptance Criteria:**
- Given `cargo check` (the prompt is `include_str!`'d), then it compiles.
- Given `cargo test lint`, then the parser/renderer tests still pass (JSON schema unchanged).
- Given the prompt, then `gaps` is framed as "a referenced entity that should be its own atomized typed note" and `under_linked` as "a missing typed edge to an existing atomized note," and it references `entity_type`/`edge_type`.
- Given the JSON keys, then `contradictions`/`stale`/`under_linked`/`gaps` and `with`/`detail`/`suggest`/`concept` are unchanged.

## Verification

- `cargo check` and `cargo test lint` — expected: compiles, lint tests pass.
