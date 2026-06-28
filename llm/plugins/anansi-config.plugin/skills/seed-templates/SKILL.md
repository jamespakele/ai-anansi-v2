---
name: seed-templates
description: >
  One-shot skill to seed the template registry into the Anansi database.
  Reads templates from the plugin's references/templates/ directory and
  writes them to the DB. Idempotent — skips templates that already exist.
  Only needed for fresh installs; after that, templates are managed via
  the database and optionally synced to the llm-wiki.
argument-hint: "[root path, default: /data]"
---

# seed-templates

Seed the template registry into the Anansi database.

This is a **setup skill** — run once on a fresh install. After that,
templates are managed through the database (add, update, delete via
`anansi-new-entity-type` or direct DB queries) and optionally synced
to `~/llm-wiki/templates/` for agent reference.

---

## When to invoke

- First-time setup of a new Anansi instance
- After a database reset
- To restore templates to factory defaults

Do **not** invoke for:
- Routine template management (use `anansi-new-entity-type` or DB)
- Server-side pipeline runs (templates are loaded from DB automatically)

---

## Step 1 — Locate templates

Templates are at `{plugin_dir}/references/templates/`. The plugin dir
is relative to this skill: `../../references/templates/`.

Read all `.md` files in that directory. Each file is one template.

---

## Step 2 — Seed to database

For each template file, call `anansi_ingest_atomized` with a single-block
atomized payload. The server stores it in the template registry.

Skip templates that already exist in the DB (check via `anansi_filter`
with `entity_type: anansi_config`).

---

## Step 3 — Report

```
*seed-templates* — {N} templates
• Created: {N}
• Skipped (already exist): {M}
• Source: anansi-config.plugin/references/templates/
```
