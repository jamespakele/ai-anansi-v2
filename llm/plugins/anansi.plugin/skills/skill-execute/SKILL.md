---
name: skill-execute
description: >
  Executes a skill with validation and retry. Reads the target skill's
  SKILL.md, executes it step by step, then runs the skill's self-check
  or success metrics. If the output doesn't pass, re-runs with the
  validation errors as feedback. Retries up to 3 times before reporting
  failure. Designed for reliable, repeatable skill execution without
  manual babysitting.
argument-hint: "[skill name] [input]"
---

# skill-execute

A harness for reliable skill execution.

Reads a skill's SKILL.md, executes it faithfully, validates the output
against the skill's own success criteria, and retries with feedback if
it doesn't pass.

---

## When to invoke

- Any time you want to run a skill and trust the result
- Instead of invoking a skill directly, use `/skill-execute <skill> <input>`
- For skills where correctness matters (atomization, extraction, etc.)

---

## Inputs

- **skill name** — the name of the skill to execute (e.g. `sb-atomize`, `para-process`)
- **input** — the input to pass to the skill (file path, URL, pasted text, etc.)

---

## Step 1 — Load the target skill

Read `{skill_name}/SKILL.md`. Parse its structure:

- **Steps** — the numbered or bulleted execution steps
- **Self-check / Success metrics** — the validation criteria (if present)
- **Output format** — what the skill should produce
- **Failure modes** — what can go wrong

If the skill has a `## Self-check` or `## Success metrics` section, extract
the checks as a checklist. If it doesn't, infer done-ness from the output:
the skill is "done" when it has produced all the files or artifacts it
describes in its output format section.

---

## Step 2 — Execute the skill

Follow the skill's steps in order. For each step:

1. **Announce** the step before executing it
2. **Execute** it faithfully — do not skip, shortcut, or improvise
3. **Capture** the output or intermediate artifact
4. **Proceed** to the next step

Do not skip any step. If a step says "read fully and follow step-XX",
read and follow step-XX. No exceptions.

---

## Step 3 — Validate the output

Run the skill's self-check or success metrics. If the skill has an
explicit checklist, check each item:

```
[ ] All required files exist
[ ] Output format matches specification
[ ] TOC structure preserved (addresses, section labels)
[ ] No entries missing or duplicated
[ ] Entity types match expected types
```

If the skill doesn't have an explicit self-check, validate against the
output format described in the skill:

- Were all required outputs produced?
- Do they match the format described?
- Are there any obvious errors (missing sections, wrong numbering, etc.)?

---

## Step 4 — Retry if validation fails

If any check fails:

1. **Collect the validation errors** — specific, actionable feedback
2. **Re-run the skill** with the errors prepended as context:

   ```
   [PREVIOUS ATTEMPT FEEDBACK]
   The following validation errors were found in the previous run:
   - Error 1: ...
   - Error 2: ...
   
   Fix these issues in this run. Pay special attention to the
   self-check criteria.
   ```

3. **Repeat** up to 3 times total
4. If still failing after 3 attempts, **report failure** with all errors

---

## Step 5 — Report

```
*skill-execute* — {skill name}
• Attempts: {N}
• Status: {passed | failed after N attempts}
• Output: {file paths or summary}
• Validation: {checks passed / total checks}

{If failed, list remaining errors}
```
