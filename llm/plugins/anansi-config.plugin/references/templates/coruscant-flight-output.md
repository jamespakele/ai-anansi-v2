---
entity_type: coruscant_flight_output
template_class: identity
atomic: true
merge_strategy: pure_atomic
template_version: "2.0"
description: "Structured output from a completed Coruscant flight — what the drone found or produced"

atomic_criteria: >
  Represents the substantive output of one flight: research findings, analysis results,
  decisions, or synthesized content. A flight-output atom is source-bound to the flight
  that produced it via source_id. Multiple flight-output atoms can exist for one flight
  when the output contains distinct named entities worth addressing individually.
  When output is monolithic (one coherent finding), one atom is correct.

identity_fields:
  name:
    type: string
    required: true
  flight_address:
    type: string
    description: "Address of the parent flight (e.g., '1.2'). The flight-output address adds a sub-index (e.g., '1.2.1', '1.2.2')."
  output_type:
    type: string
    description: "What kind of output this is: research, analysis, decision, synthesis, artifact, recommendation."
  summary:
    type: string
    description: "One-sentence summary of what this output captures."
  content:
    type: string
    format: prose
    description: "The full structured output from the flight — findings, analysis, recommendations, or artifact content."

sources:
  generic_prose:
    hint: "Flight-output records are emitted by drones at the end of their flight, not extracted from general prose."
toc_structure: "none"
---
%%
field: name
description: Name of this output atom (the finding, document, or decision it represents)
%%
%%
field: flight_address
description: Address of the parent flight (e.g., '1.2')
%%
%%
field: output_type
description: Kind of output (research, analysis, decision, synthesis, artifact, recommendation)
%%
%%
field: summary
description: One-sentence summary of what this output captures
%%
%%
field: content
description: The full structured output — findings, analysis, recommendations, or artifact content. Write as a durable, source-agnostic knowledge entry where possible.
%%
# {{name}}

## Identity
- Flight: {{flight_address}}
- Output Type: {{output_type}}

## Summary
{{summary}}

## Content
{{content}}
