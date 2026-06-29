---
name: sync-setup
description: >
  Sets up Syncthing on THIS machine (the client) to mirror the anansi
  llm-wiki cache from the VPS as a Receive-Only folder over Tailscale.
  Installs Syncthing if missing, adds the VPS as a remote device, adds the
  llm-wiki folder as receiveonly, accepts the device/folder on the server
  side (via SSH if available), and verifies the invariants: Receive-Only
  client, Tailscale-only transport (not relaying), and a sample file
  round-trips. The local cache is VIEW-ONLY — never edit files here; all
  changes re-enter through anansi (anansi-remember / wiki-sync).
argument-hint: "[--host <vps-tailscale-name>] [--user <ssh-user>]"
---

# sync-setup

**Execution protocol:** Execute every step below in order. After all steps,
run the [Done](#done) checklist. If any check fails, re-run with the errors
as feedback. Repeat until all checks pass or 3 attempts. Do not skip steps.

Configures THIS machine as a **Receive-Only** Syncthing client that mirrors
the anansi `llm-wiki` cache from the VPS. The VPS runs Syncthing Send-Only,
bound to its Tailscale interface. This machine runs Syncthing Receive-Only
and pulls over the tailnet. The cache on this machine is a read-only mirror
for Obsidian browsing — never edit it; modifications go through anansi
(`anansi-remember`, `wiki-sync`), and the next sync reflects them.

---

## Server coordinates (defaults)

These come from running `deploy/syncthing-bootstrap.py` on the VPS. If the
VPS was reprovisioned, re-run the bootstrap and update these values.

| Field | Value |
|---|---|
| VPS Tailscale name | `hostinger-vps` |
| VPS Tailscale IPv4 | `100.118.188.14` |
| VPS Syncthing device ID | `K44NMXC-V4IJG6A-PO3NPCD-TAOBEJV-WCO7KBF-D4ZUYU2-R34XIXL-BCVZRQQ` |
| Sync address | `tcp://100.118.188.14:22000` (or `tcp://hostinger-vps:22000` via MagicDNS) |
| Folder ID | `llm-wiki` |
| Folder label | `anansi llm-wiki cache` |
| Local cache path | `~/llm-wiki` |
| Server SSH (for auto-accept) | `root@hostinger-vps` |

Override the VPS host / SSH user with the argument hints if your setup differs.

---

## When to invoke

- First-time setup of cache sync on a new machine (desktop / laptop / WSL).
- After a Syncthing upgrade or config reset on this machine.
- When the user says "set up sync", "configure the cache mirror",
  "connect this machine to anansi sync", or "/sync-setup".

---

## Prerequisites

- This machine is on the same Tailscale tailnet as the VPS
  (`tailscale status` lists `hostinger-vps`). If not, run `tailscale up`
  / `tailscale login` first and stop — the whole design is Tailscale-only.
- (Optional, for auto-accept) SSH access to `root@hostinger-vps` works.
  Without it, Step 5 falls back to manual acceptance via the VPS GUI.

---

## Step 1 — Detect OS and install Syncthing if missing

Check `which syncthing`. If present, skip to Step 2.

Install per OS (run the matching command; ask the user to confirm sudo if
needed):

- **Linux (Debian/Ubuntu):**
  ```
  sudo mkdir -p /etc/apt/keyrings
  curl -fsSL -o /usr/share/keyrings/syncthing-archive-keyring.gpg https://syncthing.net/release-key.gpg
  echo "deb [signed-by=/usr/share/keyrings/syncthing-archive-keyring.gpg] https://apt.syncthing.net/ syncthing stable" | sudo tee /etc/apt/sources.list.d/syncthing.list
  sudo apt-get update && sudo apt-get install -y syncthing
  ```
- **macOS:** `brew install syncthing`
- **Windows / WSL:** WSL → use the Linux steps inside WSL. Native Windows →
  install via `scoop install syncthing` or `winget install Syncthing.Syncthing`,
  then run as a service.

Enable + start the systemd user service (Linux) so it runs without a login
session:
```
sudo loginctl enable-linger $USER   # only if not already lingering
systemctl --user enable --now syncthing.service
```
(macOS: `brew services start syncthing`. Windows: set the service to auto-start.)

Wait a few seconds, then confirm `syncthing --version` and that the GUI is up
on `127.0.0.1:8384` (`ss -tlnp | grep 8384` on Linux, or check the tray icon).

---

## Step 2 — Read this machine's Syncthing identity + API key

- Local config dir: `~/.local/state/syncthing` (Linux), `~/Library/Application Support/Syncthing` (macOS), `%LOCALAPPDATA%/Syncthing` (Windows).
- Device ID: `syncthing -device-id -home <config-dir>` (or read the `<device id="...">` from `config.xml`).
- API key: the `<gui><apikey>...</apikey></gui>` value in `config.xml`. If
  absent, generate one: stop Syncthing, add `<apikey><random 32-char></apikey>`
  inside `<gui>`, restart.

Record both — Step 3 and 4 use the API key to drive the local REST API at
`http://127.0.0.1:8384`.

---

## Step 3 — Add the VPS as a remote device on THIS machine

Use the local REST API (X-API-Key header from Step 2):

```
curl -s -X POST http://127.0.0.1:8384/rest/config/devices \
  -H "X-API-Key: <local-apikey>" -H "Content-Type: application/json" \
  -d '{
    "deviceID": "K44NMXC-V4IJG6A-PO3NPCD-TAOBEJV-WCO7KBF-D4ZUYU2-R34XIXL-BCVZRQQ",
    "name": "anansi-vps",
    "addresses": ["tcp://100.118.188.14:22000"],
    "introducer": false,
    "paused": false,
    "autoAcceptFolders": false
  }'
```

If a device with that ID already exists (re-run), use `PUT
/rest/config/devices/K44NMXC-...` with the same body instead of POST.

Confirm: `curl -s http://127.0.0.1:8384/rest/config/devices -H "X-API-Key: ..."`
lists the VPS device with the Tailscale address (not `dynamic`, not a relay).

---

## Step 4 — Add the llm-wiki folder as Receive-Only on THIS machine

Ensure the local cache path exists: `mkdir -p ~/llm-wiki`.

```
curl -s -X POST http://127.0.0.1:8384/rest/config/folders \
  -H "X-API-Key: <local-apikey>" -H "Content-Type: application/json" \
  -d '{
    "id": "llm-wiki",
    "label": "anansi llm-wiki cache",
    "path": "<absolute ~/llm-wiki path>",
    "type": "receiveonly",
    "rescanIntervalS": 3600,
    "devices": [
      {"deviceID": "<THIS machine device ID>"},
      {"deviceID": "K44NMXC-V4IJG6A-PO3NPCD-TAOBEJV-WCO7KBF-D4ZUYU2-R34XIXL-BCVZRQQ"}
    ]
  }'
```

`type: "receiveonly"` is the load-bearing invariant: this machine never
pushes wiki content back. If the folder already exists, `PUT
/rest/config/folders/llm-wiki` instead.

**Simple File Versioning** — keep 5 versions, clean out after 7 days.
This prevents `.sync-conflict-*` clutter if a conflict somehow occurs:

```
curl -s -X PATCH http://127.0.0.1:8384/rest/config/folders/llm-wiki \
  -H "X-API-Key: <local-apikey>" -H "Content-Type: application/json" \
  -d '{"versioning": {"type": "simple", "params": {"keep": "5", "cleanoutDays": "7"}}}'
```

If the REST API doesn't support PATCH, use the CLI instead:
```
syncthing cli config folders llm-wiki versioning type simple
syncthing cli config folders llm-wiki versioning params keep 5
syncthing cli config folders llm-wiki versioning params cleanoutDays 7
```

---

## Step 5 — Accept the device + share the folder on the VPS side

The VPS won't auto-accept (auto-accept is off by design). Two paths:

**A. Auto-accept via SSH (preferred, if `ssh root@hostinger-vps` works):**
The VPS Syncthing GUI/API is at `127.0.0.1:8384` (localhost only). Tunnel it
and use the server's API key (from `/root/.local/state/syncthing/config.xml`
on the VPS):

```
# 1. Add THIS machine as a device on the VPS (replace <THIS-DEVICE-ID>)
ssh root@hostinger-vps 'curl -s -X POST http://127.0.0.1:8384/rest/config/devices \
  -H "X-API-Key: <server-apikey>" -H "Content-Type: application/json" \
  -d "{\"deviceID\":\"<THIS-DEVICE-ID>\",\"name\":\"<this-hostname>\",\"addresses\":[\"dynamic\"],\"autoAcceptFolders\":false}"'

# 2. Share the llm-wiki folder with THIS device on the VPS
ssh root@hostinger-vps 'curl -s -X PUT http://127.0.0.1:8384/rest/config/folders/llm-wiki \
  -H "X-API-Key: <server-apikey>" -H "Content-Type: application/json" \
  -d "{\"id\":\"llm-wiki\",\"devices\":[{\"deviceID\":\"K44NMXC-V4IJG6A-PO3NPCD-TAOBEJV-WCO7KBF-D4ZUYU2-R34XIXL-BCVZRQQ\"},{\"deviceID\":\"<THIS-DEVICE-ID>\"}]}"'
```

Use `"dynamic"` for the client's address on the server side — the client is
reachable over Tailscale and the server will discover it; you do NOT need to
expose the client's sync port publicly.

**B. Manual (no SSH):** Open the VPS GUI via SSH tunnel
(`ssh -L 8384:127.0.0.1:8384 root@hostinger-vps` then browse
`http://localhost:8384`), accept the pending device, and share the
`llm-wiki` folder with it.

---

## Step 6 — Verify the invariants

Wait ~30s for the connection + first scan, then check each:

1. **Connection is over Tailscale, not a relay.**
   `curl -s http://127.0.0.1:8384/rest/system/connections -H "X-API-Key: ..."`
   → the VPS device's `address` should be `tcp://100.118.188.14:22000`
   (or the Tailscale IP/hostname), and `connected: true`. If it shows
   `relay://...` or is disconnected, Tailscale isn't connecting — fix that
   before proceeding (do NOT enable public relaying as a workaround; that
   defeats the design).

2. **Folder is Receive-Only on this machine.**
   `curl -s http://127.0.0.1:8384/rest/config/folders/llm-wiki -H "X-API-Key: ..."`
   → `"type": "receiveonly"`. If anything else, fix it: the client must never
   push wiki content.

3. **A sample file round-trips.** The cache currently holds entity markdown
   files (e.g. `andrej-karpathy.person.md`). Confirm at least one appeared
   in `~/llm-wiki/`:
   `ls ~/llm-wiki/*.md | head`. If the cache is empty on the VPS (e.g. the
   anansi container hasn't written yet), create a probe on the VPS:
   `ssh root@hostinger-vps 'echo probe > /root/llm-wiki/.sync-probe'`,
   wait for it to arrive here, then delete it from the VPS — it will
   propagate the delete here too (Receive-Only accepts deletes from the
   source).

4. **No public sync port on this client.** Confirm Syncthing on this machine
   is listening on its Tailscale IP (or localhost) for 22000, not `0.0.0.0`
   exposed to the LAN/internet: `ss -tlnp | grep 22000` (Linux). If it's on
   `0.0.0.0`, set the client's `listenAddress` to its Tailscale IP and
   restart, mirroring the server-side discipline.

5. **Simple File Versioning is configured.**
   `syncthing cli config folders llm-wiki`
   → `"type": "simple"` with `keep: 5` and `cleanoutDays: 7`.

---

## Step 7 — Clean up stale sync-conflict files

If this machine was previously synced without versioning, stale
`.sync-conflict-*` files may have accumulated. Remove them:

```
find ~/llm-wiki -name '*.sync-conflict*' -type f -delete
echo "Removed $(find ~/llm-wiki -name '*.sync-conflict*' -type f | wc -l) conflict files"
```

These are safe to delete — the originals are in the folder and the entities
are in the Anansi database. With Simple File Versioning now enabled, future
conflicts will be archived to `.stversions/` and auto-cleaned after 7 days.

---

## Step 8 — Report

```
*sync-setup* — <this hostname>
• Syncthing: <version>
• Local device ID: <this device id>
• Remote (VPS) device ID: K44NMXC-V4IJG6A-PO3NPCD-TAOBEJV-WCO7KBF-D4ZUYU2-R34XIXL-BCVZRQQ
• Folder: llm-wiki → ~/llm-wiki (receiveonly)
• Versioning: simple (keep 5, cleanout 7d)
• Transport: Tailscale (100.118.188.14:22000) — relay: no
• Sample round-trip: pass
• Reminder: ~/llm-wiki is VIEW-ONLY. Edits via anansi-remember / wiki-sync.
```

## Done

- [ ] Syncthing installed and running on this machine (auto-start on boot)
- [ ] This machine on the same Tailscale tailnet as the VPS
- [ ] VPS device added on this machine with the Tailscale address (not `dynamic`/relay)
- [ ] `llm-wiki` folder added on this machine with `type: receiveonly`
- [ ] Simple File Versioning configured (keep 5, cleanout 7 days) on both sides
- [ ] Stale `.sync-conflict-*` files cleaned up
- [ ] Connection is over Tailscale (`connected: true`, address is the Tailscale IP, no relay)
- [ ] Folder type is `receiveonly` on this machine (verified via REST, not just GUI)
- [ ] Sample file round-trips (or probe created+deleted successfully)
- [ ] Client sync port is NOT exposed publicly (Tailscale/localhost only)
- [ ] User reminded: `~/llm-wiki` is view-only; edits go through anansi

---

## Notes

- **Why Receive-Only matters:** anansi is the single writer of wiki content.
  A Receive-Only client physically cannot push local edits back, which
  preserves the single-writer invariant even if a tool "helpfully" edits a
  file locally. Syncthing will flag such local changes as out-of-sync items
  to revert, not propagate them.
- **Why Tailscale-only matters:** the VPS sync port is bound to its Tailscale
  interface and is not exposed to the public internet. If a client ever
  connects via a public Syncthing relay, something is wrong with Tailscale
  on one side — fix the tailnet, do not enable public relaying.
- **Reprovisioning the VPS:** if the VPS is rebuilt, the device ID changes.
  Re-run `deploy/syncthing-bootstrap.py` on the new VPS and update the
  server coordinates at the top of this skill.
- **Staleness:** the cache reflects the last sync, not live server state. A
  `remember` on this machine hits anansi; the new entity appears here after
  the next sync cycle. For faster feedback, trigger a manual rescan or
  restart Syncthing after a remember session.