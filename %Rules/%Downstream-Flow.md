---
id: downstream-flow
type: rule
status: active
version: "2.0"
last_updated: 2026-04-23
---

# Downstream Flow Rule

## What "Downstream" Means

Every source document contains two kinds of content about an entity:

1. **Identity content** — facts that are true about the entity regardless
   of this source. These belong in the entity's own note.

2. **Source-specific content** — what this particular document says about
   the entity. These belong downstream in a context, event, or task node.

## The Routing Contract

When the TOC includes a `context_at` annotation on a leaf, it specifies
the TOC addresses of the context nodes that should receive the source-specific
content about that entity. The Pass 3 prompt enforces this:

> Content about {entity_name} that is specific to this source belongs in
> context nodes at {context_at_addresses}. Do NOT duplicate that content
> in the {entity_type} note.

## Examples

**Person note** (`-ian-kitajima.md`):
- ✅ Name, contact email, one-sentence bio
- ❌ What Ian said at the Digital Futures Workshop (→ context node)
- ❌ The project Ian is leading this quarter (→ context/task node)

**Concept note** (`-sovereign-ai.md`):
- ✅ Definition of Sovereign AI, related concepts
- ❌ How the workshop discussed Sovereign AI (→ context node 3.2)
- ❌ The policy position taken in the email thread (→ context node)

**Organization note** (`-pichtr.md`):
- ✅ Org name, type, domain, summary, roster of people
- ❌ What PICHTR presented at the workshop (→ context node)
- ❌ PICHTR's current budget for this project (→ context/task node)

## The `context_at` Annotation

In the preprocessed-TOC schema, identity-type leaves may carry:

```
1.1 Ian Kitajima [person] | hint: CTO of PICHTR | context_at: 3.1, 3.2
```

This tells the daemon and the Cowork skill: "Content about Ian from this
source belongs in context nodes 3.1 and 3.2, not in the person note."

If `context_at` is absent, Pass 3 still enforces purity — it just lacks
the explicit routing addresses and must infer from context.
