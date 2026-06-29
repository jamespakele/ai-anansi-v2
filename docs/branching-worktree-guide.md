# Branching & Worktree Guide

## Overview

This repo uses git branches for feature development. Branches share one working folder — switching branches rewrites the files in place. For parallel development (e.g., building a new layer while keeping `main` running), use `git worktree` to create a separate working folder.

## Current setup

| Folder | Branch | Purpose |
|---|---|---|
| `ai-anansi-v2/` | `main` | The working anansi — source of truth |
| `ai-anansi-v2-llm-wiki/` | `feat/llm-wiki` | LLM-wiki layer workspace |

Both share the same `.git` database. Commits in either folder land in the right branch automatically.

## Basic branching workflow

### Create a new branch (single folder)

```sh
git checkout -b feat/<short-name>
```

### Switch between branches

```sh
git checkout main        # back to main
git checkout feat/xxx    # to your feature branch
```

### Push a branch to origin

```sh
git push -u origin feat/<short-name>
```

### Merge a feature branch back into main

```sh
git checkout main
git pull
git merge --no-ff feat/<short-name>
git push
```

`--no-ff` preserves a merge commit so the history shows "this whole branch was the update."

## Worktree workflow (separate folders)

### Create a new worktree

```sh
# From inside ai-anansi-v2/
git worktree add ../ai-anansi-v2-<layer> -b feat/<branch-name>
```

This creates a sibling folder with a new branch at the current commit.

### List all worktrees

```sh
git worktree list
```

### Remove a worktree (after merging or discarding)

```sh
git worktree remove ../ai-anansi-v2-<layer>
git branch -d feat/<branch-name>   # or -D to force discard
```

## Important notes

- **Separate vault data.** `vault/`, `target/`, and any data path configured via a relative path in `anansi.toml` are independent per worktree. If you want two worktrees to share the same data, configure an absolute path — but don't run two anansi instances writing to the same DB simultaneously.
- **`target/` is per-worktree.** Rust build artifacts are not shared. First build in a new worktree compiles from scratch.
- **Open a worktree as a separate Zed project.** The agent tooling is scoped to whichever project is open.
- **No tags or release branches exist** in this repo currently.
