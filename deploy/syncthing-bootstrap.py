#!/usr/bin/env python3
"""
Idempotent Syncthing configuration for the anansi llm-wiki cache sync.

Architecture (see design notes in the repo):
  - anansi (container) is the SOLE WRITER of the cache.
  - This machine runs Syncthing as a SEND-ONLY source over the Tailscale
    interface only. Public sync port is NOT exposed.
  - Client machines run Syncthing RECEIVE-ONLY and pull from this source.
  - The cache directory is a derived, rebuildable projection of the anansi
    vault; Syncthing never writes wiki content, only its own .stfolder marker.

This script configures the SERVER side. It is idempotent: re-running after a
Syncthing upgrade or config drift restores the intended state without
re-provisioning the host (Syncthing itself must already be installed and the
syncthing@<user> systemd service enabled).

Run as the same user that owns the Syncthing config and the cache directory
(typically root on the VPS, since /root is mode 0700 and the cache lives at
/root/llm-wiki). Requires `tailscale` on PATH to discover the bind address.

Env overrides:
  SYNCTHING_CONFIG   path to config.xml   (default: ~/.local/state/syncthing/config.xml)
  ANANSI_WIKI_PATH   cache directory       (default: ~/llm-wiki)
  SYNCTHING_SERVICE  systemd unit          (default: syncthing@root.service)
"""

from __future__ import annotations

import copy
import os
import subprocess
import sys
import xml.etree.ElementTree as ET

CONFIG = os.environ.get(
    "SYNCTHING_CONFIG", os.path.expanduser("~/.local/state/syncthing/config.xml")
)

CACHE = os.environ.get("ANANSI_WIKI_PATH") or os.path.expanduser("~/llm-wiki")
SERVICE = os.environ.get("SYNCTHING_SERVICE", "syncthing@root.service")
FOLDER_ID = "llm-wiki"
FOLDER_LABEL = "anansi llm-wiki cache"


def die(msg: str) -> "None":
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


def run(cmd: list[str]) -> str:
    return subprocess.check_output(cmd, text=True).strip()


def tailscale_ipv4() -> str:
    try:
        return run(["tailscale", "ip", "-4"])
    except Exception as e:
        die(f"could not determine Tailscale IPv4 via `tailscale ip -4`: {e}")
        return ""  # unreachable


def set_text(parent: ET.Element, tag: str, value: str) -> None:
    el = parent.find(tag)
    if el is None:
        el = ET.SubElement(parent, tag)
    el.text = value


def main() -> None:
    if os.geteuid() != 0 and SERVICE.endswith("@root.service"):
        print(
            "WARNING: not running as root but service is syncthing@root.",
            file=sys.stderr,
        )

    if not os.path.exists(CONFIG):
        die(
            f"Syncthing config not found at {CONFIG}. Is Syncthing installed and started once?"
        )

    ts_ip = tailscale_ipv4()
    print(f"Tailscale IPv4: {ts_ip}")

    # Stop Syncthing before editing (it rewrites config.xml on shutdown).
    print(f"Stopping {SERVICE} ...")
    subprocess.run(["systemctl", "stop", SERVICE], check=False)

    tree = ET.parse(CONFIG)
    root = tree.getroot()

    # --- Global options: bind to Tailscale only, drop discovery/relay/NAT ---
    opts = root.find("options")
    if opts is None:
        die("config has no <options> element")
    set_text(opts, "listenAddress", f"tcp://{ts_ip}:22000")
    set_text(opts, "globalAnnounceEnabled", "false")
    set_text(opts, "localAnnounceEnabled", "false")
    set_text(opts, "relaysEnabled", "false")
    set_text(opts, "natEnabled", "false")

    # --- Local device id (first <device> child of <configuration>) ---
    local_dev_el = root.find("./device")
    if local_dev_el is None:
        die("config has no local <device> element")
    local_dev_id = local_dev_el.get("id")
    print(f"Local device ID: {local_dev_id}")

    # --- Remove the default sample folder and any prior llm-wiki folder (idempotent) ---
    for f in list(root.findall("folder")):
        if f.get("id") in ("default", FOLDER_ID):
            root.remove(f)

    # --- Build the llm-wiki folder from the <defaults>/<folder> template ---
    tmpl = root.find("defaults/folder")
    if tmpl is None:
        die("config has no <defaults>/<folder> template to clone")
    new_folder = copy.deepcopy(tmpl)
    new_folder.set("id", FOLDER_ID)
    new_folder.set("label", FOLDER_LABEL)
    new_folder.set("path", CACHE)
    new_folder.set("type", "sendonly")  # Send-Only: this machine is the source
    dev = new_folder.find("device")
    if dev is not None:
        dev.set("id", local_dev_id)
        dev.set("introducedBy", "")
    # Insert as the first child (folders precede <device> in the schema)
    root.insert(0, new_folder)

    # --- Ensure the cache directory exists (Syncthing needs a real path) ---
    os.makedirs(CACHE, exist_ok=True)
    # Syncthing writes a .stfolder marker; allow it by ensuring the dir is writable by this user.
    print(f"Cache directory: {CACHE}")

    # --- Write config back ---
    tree.write(CONFIG, encoding="utf-8", xml_declaration=True)
    print(f"Wrote {CONFIG}")

    # --- Start Syncthing ---
    print(f"Starting {SERVICE} ...")
    subprocess.run(["systemctl", "start", SERVICE], check=True)

    # --- Verify ---
    print("\n--- verification ---")
    try:
        listening = run(["sh", "-c", "ss -tlnp 2>/dev/null | grep 22000 || true"])
        print(f"listen 22000: {listening or '(not yet bound — check again shortly)'}")
    except Exception:
        pass
    print(f"Tailscale sync address: tcp://{ts_ip}:22000")
    print(f"Device ID: {local_dev_id}")
    print(f"Folder: {FOLDER_ID} ({CACHE}) type=sendonly")
    print(
        "\nNext: on each client, run the `sync-setup` skill (Receive-Only), "
        "add this device by its ID, and share folder id 'llm-wiki'."
    )


if __name__ == "__main__":
    main()
