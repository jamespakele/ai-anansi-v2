# Troubleshooting — 2026-06-28 VPS deploy & skills incidents

Record of the issues hit during the `feat/llm-wiki` → `main` deploy and the
consolidation onto the canonical `llm/plugins` skills path. Each entry: symptom,
root cause, fix, and verification. Commits referenced by Build number.

---

## 1. Deploy failed: db host port 5432 conflict with supabase

**Symptom:** After merging `feat/llm-wiki` to `main`, the Hostinger deploy
cloned the repo, built the image, but `docker compose up` failed and left both
containers in `Created` state. anansi restart-looped with:

```
Error: error communicating with database: failed to lookup address information: Name does not resolve
```

**Root cause:** The anansi db service published `127.0.0.1:5432:5432`, but the
Supabase stack's `supabase-db` already owns `127.0.0.1:5432` on the same VPS.
The deploy's db start failed:

```
Bind for 127.0.0.1:5432 failed: port is already allocated
```

The db never attached to the compose network → anansi couldn't resolve `db` →
restart loop. "CI green" was irrelevant: the GHCR publish job succeeds, but the
Hostinger `deploy-on-vps` action is what runs `docker compose up` on the box,
and that's where it broke.

**Fix (Build-26, `9dded2d` on `main`):** Move the anansi db host bind to
`127.0.0.1:5433:5432` (container still listens on 5432; only the host-side SSH
tunnel port moves). `DATABASE_URL=…@db:5432` is unchanged — that's the
container-internal port, not the host bind. Keep supabase on 5432 untouched.

**Verification:** `ss -tlnp` shows anansi db on `127.0.0.1:5433`, supabase on
`127.0.0.1:5432`, both healthy. anansi `Up (healthy)`, `restarts=0`.

**Lesson:** On a shared VPS, never assume `5432` is free — check
`docker ps --format '{{.Ports}}'` for the port before assigning. Prefer a
non-default host port for any stack that shares a host with other postgres
instances.

---

## 2. Stuck Hostinger deploy — buildkit `runc` hang blocked the queue

**Symptom:** The Hostinger dashboard spun forever on "Your project is being
deployed." The anansi deploy (triggered by the `main` push) never started —
`.build.log` was stale, no anansi clone in `/tmp/hstgr-*`.

**Root cause:** A *different* app's deploy (`recipe-base`) was occupying the
Hostinger docker-mgr, which serializes deploys. Its `docker compose up
--force-recreate --build --no-start` process had finished building (images
existed) but the wrapper + a buildkit `runc` process hung as zombies (~10+ min,
`Sl` state, doing nothing). The recurring `runc-log.json` errors
("container does not exist", "broken pipe") indicate buildkit runc flakiness.
The queued anansi deploy couldn't start until the zombie exited.

**Fix:** Identified the hung PIDs (`docker compose … recipe-base … up` and its
`runc` child), confirmed `recipe-base`'s own containers were already `Up`
(deploy had effectively succeeded), then `kill` / `kill -9` the zombies. That
freed the docker-mgr, which immediately cloned anansi and ran the deploy.

**Verification:** A fresh `/tmp/hstgr-*-dckr-mgr` clone appeared, `.build.log`
went live ("Image ai-anansi-v2-anansi Building" → "Project deployed
successfully"), anansi recreated healthy.

**Lesson:** The Hostinger docker-mgr is single-flight; one stuck deploy blocks
all queued deploys and spins the dashboard. On a stuck "being deployed" state,
check `ps aux | grep -E 'docker compose|buildkit|runc'` for zombie build
processes; if the target app's containers are already up, killing the hung
`up` wrapper is safe and unblocks the queue. The deeper fix is to stop building
on the VPS at all (see §6).

---

## 3. `~/llm-wiki` bind mount didn't expand (no `HOME` in the deploy env)

**Symptom:** A literal directory named `~` appeared at
`/docker/ai-anansi-v2/~/llm-wiki` instead of the cache landing at
`/root/llm-wiki`. The build log warned:

```
cannot expand '~', because the environment lacks HOME
```

**Root cause:** The compose had `- ~/llm-wiki:/data/llm-wiki`. Docker Compose
expands `~` using the *host process's* `HOME`. The Hostinger deploy runs
`docker compose` in an environment with **no `HOME` set**, so `~` isn't
expanded → Compose creates a literal `~` directory relative to the compose dir.
(Inside the container, anansi's own `~` expansion in `wiki.dir` is separate and
uses the container's HOME — that path was fine; the bug was the host-side bind.)

**Fix (Build-26):** Removed the `~/llm-wiki:/data/llm-wiki` bind mount entirely.
The cache already persists via the existing `./data:/data` mount — anansi writes
to `/data/llm-wiki` (container) = `./data/llm-wiki` (host) = under the project
dir, no `~` involved, no scatter outside `/docker/ai-anansi-v2`.

**Lesson:** Never use `~` in a production compose bind-mount path — deploy
environments often lack `HOME`. Use an absolute path, a path under an existing
named/relative mount, or an explicit env var. Keep all app files inside the
project dir for housekeeping on a shared VPS.

---

## 4. Empty `/data/skills` — inbox watcher running with no skills

**Symptom:** On the VPS, `docker exec … ls /data/skills | wc -l` returned `0`.
The inbox watcher was running with **no skills loaded**, silently.

**Root cause (two compounding issues):**

1. The prod compose bind-mounted `./claude-cowork/plugins/anansi.plugin/skills`
   → `/data/skills`. But `claude-cowork` is gitignored, and the Hostinger
   deploy **only syncs `docker-compose.yml`, `Dockerfile`, `.env`,
   `.env.example`** to `/docker/ai-anansi-v2 — it does NOT sync the skills tree
   (no `llm/`, no `src/`). So the bind-mount source was an absent path → Docker
   mounted an empty dir → `/data/skills` empty, shadowing the skills baked into
   the image at `/app/skills`.
2. The VPS `anansi.toml` set `skills_dir = "/data/skills"`, so anansi read the
   empty mount instead of the baked `/app/skills` (which has 26 skills, baked
   from `llm/plugins` via the Dockerfile).

**Fix (two parts):**

- **VPS runtime config (applied on the box):** Set `skills_dir = "/app/skills"`
  in `/docker/ai-anansi-v2/data/anansi/anansi.toml` (backed up first), then
  `docker restart ai-anansi-v2-anansi-1`. anansi now reads the baked skills.
  This is a persistent runtime config in the `./data` volume — it is NOT
  rebuilt by deploys, so the change survives.
- **Repo (`fix/skills-from-llm-plugins`, `6c2579b`):** Removed the dead skills
  bind mount from the prod compose (prod uses the image-baked `/app/skills`;
  `default_skills_dir()` is already `/app/skills`). Added
  `./llm/plugins/anansi.plugin/skills:/data/skills:ro` to the **dev override**
  (`docker-compose.local.yml`) so local dev keeps live skill editing (the repo
  is present locally; `skills_dir = "/data/skills"` for dev). Updated
  `anansi.toml.example`, `README.md`, and `src/config.rs`/`src/inbox.rs`
  comments to reflect baked-vs-dev and drop stale `claude-cowork`/`r2-anansi`
  references.

**Verification:** Post-restart logs show `[inbox] watcher started … (skills:
/app/skills, …)` (was `skills: /data/skills`). `docker exec … ls /app/skills |
wc -l` = 26. anansi `Up (healthy)`.

**Lesson:** On a deploy that syncs only a subset of files, **bind-mounting a
repo path that isn't synced gives you an empty mount that silently shadows
image-baked content.** For prod, prefer image-baked assets (the image is built
from the full repo clone); reserve bind mounts for local dev where the working
tree is present. Always verify `docker exec … ls <skills_dir>` after deploy,
not just the healthcheck (a port-up healthcheck passes even with empty skills).

---

## 5. Duplicate skills paths — `claude-cowork` → `llm/plugins` consolidation

**Context:** Skills originally lived in `claude-cowork/plugins/anansi.plugin/
skills` (an external, Claude-workspace-specific path, not version-controlled
with the codebase). They were moved into the repo at `llm/plugins/anansi.plugin/
skills` to be LLM-agnostic and version-controlled. `claude-cowork` was left as a
gitignored mirror and was the bind-mount source — which is what made §4 happen.

**Fix:** Consolidated on `llm/plugins` as canonical:
- `claude-cowork/` removed locally (verified via `diff -rq` to be a pure mirror
  of `llm/plugins`) and on the VPS (stale empty leftover).
- Local anansi restarted on the fix-branch compose so `/data/skills` mounts from
  `llm/plugins` (not `claude-cowork`), then `claude-cowork` deleted.
- `.gitignore` keeps `/claude-cowork` (defensive — if it ever reappears, it
  stays ignored and won't deploy).

**Lesson:** When migrating a canonical path, remove the old path from the
*runtime mount reference* and the *deploy sync*, not just from git — a
gitignored dir that's still bind-mounted in prod is a silent failure waiting to
happen (§4).

---

## 6. Deploy pre-built GHCR images (DONE — landed via `chore/ghcr-deploy` + the `d6e6211` prefix fix; see §8 for the incident)

**Problem:** The VPS builds the anansi image from source on every deploy
(`build: context: .` in the compose). This is the slow, CPU-heavy path and the
source of the buildkit `runc` hangs (§2). It also duplicates work — the
`publish` CI job already builds and pushes the image to GHCR.

**In-progress (`chore/ghcr-deploy`, `5b552ef`, not yet merged):**
- Prod compose switches `build:` → `image: ghcr.io/jamespakele/anansi2:${GIT_SHA}`
  (full-sha tag; unique per commit so Docker always pulls fresh — no
  `pull_policy` needed, avoiding the VPS Compose interpolation gotcha).
- `deploy.yml` `publish` job emits a full-sha tag (`type=sha,format=long`) to
  match `${GIT_SHA}` (the `deploy` job already injects it via the `.env`).
- Dev override keeps `build:` (`image: anansi-dev`) so local dev compiles
  locally instead of pulling the prod image.
- VPS already has `ghcr.io` auth in `/root/.docker/config.json`.

**Prerequisite before merge:** the VPS's GHCR token must have **read access to
the `jamespakele/anansi2` package** (GHCR perms are per-package; recipe-base's
token covers `crabby-apps/*`, not `anansi2`). Without this, the pull 401s.

**Lesson (from the recipe-base best-practices doc):** build once in CI, pull on
the VPS — no compilation, no source, no build toolchain on the box. Removes the
entire class of on-box buildkit hangs and aligns with "the image is the unit of
deploy."

---

## 7. Cache path & the `llm-wiki` feature (still pending)

**Design landed (not yet enabled):** The anansi llm-wiki cache is a file-native
projection of the knowledge graph. On the VPS it should live at
`/docker/ai-anansi-v2/data/llm-wiki` (via `./data:/data` — see §3) — inside the
project dir, no scatter. Syncthing (Send-Only, Tailscale-only) mirrors it to
client machines (Receive-Only) — see `feat/sync` and `deploy/syncthing-bootstrap.py`.

**Still pending to actually populate the cache:**
- `[wiki] enabled = true` and `dir = "/data/llm-wiki"` in the VPS `anansi.toml`
  (the feature is opt-in; default `enabled = false`, default `dir = "~/llm-wiki"`
  which expands to the container's HOME and is *not* on the bind mount). Until
  this is set, the cache stays empty and Syncthing has nothing to sync.
- The `[wiki]` section is absent from the current VPS `anansi.toml` (it predates
  the feature).

---

## 8. GHCR tag-prefix incident — `sha-` prefix vs bare `${GIT_SHA}` (2026-06-28)

**Symptom:** After merging `chore/ghcr-deploy` (switch the VPS from local
build to pulling `ghcr.io/jamespakele/anansi2:${GIT_SHA}`), the VPS deploy
failed:

```
Image ghcr.io/jamespakele/anansi2:6df3f3a6ef… Pulling
… failed to resolve reference … not found
Project build failed
```

The VPS tried to pull the bare full-sha tag `<sha>` but it didn't exist. The
previous containers kept running (healthy), so production stayed up — only the
new deploy failed.

**Root cause:** `deploy.yml`'s `type=sha,format=long` was missing `prefix=`.
`docker/metadata-action`'s default prefix for `type=sha` is `sha-`, so the
published tag was `sha-<fullsha>` — but the compose references `${GIT_SHA}`
(the bare full sha, no prefix). Tag mismatch → 404. (The `chore/ghcr-deploy`
branch dropped `prefix=` when it changed `format=short` → `format=long`; the
original `type=sha,prefix=,format=short` had `prefix=` to suppress the prefix.)

**The misleading part — GH Actions showed `completed success`:** The
`6df3f3a` run is green in GitHub Actions even though its VPS deploy failed.
`publish` succeeded (it pushed `sha-<sha>` + `:latest`), and the `deploy`
job's `hostinger/deploy-on-vps@v2` action reports success on the API POST
being accepted (HTTP 2xx), NOT on the VPS container state. So the green check
marked "deploy request accepted," not "image pulled + container running."
This is the same class as recipe-base's Incident 9 (action green, site frozen).

**Fix (`d6e6211`):** Add `prefix=` back: `type=sha,prefix=,format=long` → the
tag is the bare full sha, matching `${GIT_SHA}`. Verified: the `d6e6211` deploy
pulled `ghcr.io/jamespakele/anansi2:d6e62116e80e63d69ad36c376c79416c73394396`
and the container now runs that image (`Up (healthy)`, reading `/app/skills`).

**Lesson:** The image tag the compose asks the VPS to pull must **exist on
GHCR and exactly match** — including prefix. Verify deploys by checking the
VPS `.build.log` + the container's `Config.Image` (or the Hostinger VPS API),
NOT the GitHub Actions green check (recipe-base Best Practice 5 / Incident 9).

---

## Key takeaways

- **"CI green" ≠ "deployed."** The GHCR publish job and the Hostinger VPS deploy
  are separate; a green publish doesn't mean the VPS containers are up.
- **GH Actions "success" ≠ VPS deployed** — `hostinger/deploy-on-vps` reports
  success on the API POST (HTTP 2xx), not the container state. The `6df3f3a`
  run was green while its VPS pull 404'd. Verify via the VPS `.build.log` +
  `docker inspect … Config.Image` (or the Hostinger VPS API).
- **GHCR tag prefix must match `${GIT_SHA}`** — `docker/metadata-action`
  `type=sha` defaults to a `sha-` prefix; use `prefix=` to suppress it if the
  compose references the bare sha (§8).
- **The Hostinger deploy syncs only a subset of files** (compose, Dockerfile,
  `.env`, `.env.example`) to `/docker/ai-anansi-v2`. It does NOT sync the repo
  tree — so bind-mounting repo paths that aren't in that subset yields empty
  mounts. Use image-baked assets for prod; bind-mount only for local dev.
- **Never use `~` in a prod compose bind-mount** — deploy envs often lack `HOME`.
- **Check ports before binding** on a shared VPS; `5432` is commonly taken.
- **The Hostinger docker-mgr is single-flight** — one stuck deploy blocks the
  queue and spins the dashboard. Look for zombie `docker compose`/`runc`
  processes.
- **Verify behavior, not just healthchecks** — a port-up healthcheck passes with
  empty skills / empty cache. `docker exec … ls <dir>` after deploy.
- **Migrating a canonical path** requires removing the old path from runtime
  mount references and deploy sync, not just from git.
- **Build on the VPS only invites flakiness** — prefer pre-built GHCR images
  pulled on the VPS (§6).