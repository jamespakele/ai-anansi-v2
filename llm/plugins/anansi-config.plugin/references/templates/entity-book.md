---
entity_type: book
template_class: identity
atomic: true
merge_strategy: title_author
template_version: "1.0"
description: >
  A nonfiction or fiction book — a standalone published work with a title,
  author, and publisher. Used as a Resource entity when a book is referenced
  in source material. Distinct from a note (which is an internal vault
  document) and from book_chapter / book_section (which are content_unit
  floor types for atomizing the book's internal structure). Use when the
  source mentions a book by title, or when the source IS a book being
  processed through the pipeline.
identity_fields:
  title:
    type: string
    required: true
    description: "Full title of the book, including subtitle"
  author:
    type: string
    required: true
    description: "Author(s) — last name, first name format"
  year:
    type: string
    description: "Publication year (four-digit)"
  publisher:
    type: string
    description: "Publisher name"
  isbn:
    type: string
    description: "ISBN-13 if known"
sources:
  book:
    hint: >
      When the source being processed IS this book, use the frontmatter
      title, author, and publisher fields verbatim. When the book is
      referenced inside another source, extract as much identity
      information as is available.
  meeting_notes:
    hint: >
      Capture the title and author as mentioned. Don't infer publication
      year unless stated.
  email:
    hint: >
      Title and author only — don't add fields not mentioned in the email.
  research_paper:
    hint: >
      Use the citation data (author, year, publisher) if present in the
      bibliography or footnotes.
toc_structure: "none"
---

%%
field: title
description: Full title of the book, including subtitle
format: prose
constraints: "Verbatim from cover or frontmatter. Include subtitle after em dash or colon."
%%

%%
field: author
description: Author(s) — last name, first name
format: prose
constraints: "One author per line if multiple. Last, First format."
%%

%%
field: year
description: Publication year
format: prose
constraints: "Four-digit year. Use first edition year if multiple editions."
%%

%%
field: publisher
description: Publisher name
format: prose
constraints: "Publisher name only — omit imprint parent unless relevant."
%%

%%
field: isbn
description: ISBN-13 if known
format: prose
constraints: "13-digit ISBN with hyphens, or omit if not available."
%%

%%
field: description
description: One-sentence description of what the book is about and why it matters
format: prose
constraints: "One sentence. What is the core claim or method, and who should read it."
%%

# {{title}}

**Author:** {{author}}
**Year:** {{year}}
**Publisher:** {{publisher}}
**ISBN:** {{isbn}}

## Description
{{description}}
