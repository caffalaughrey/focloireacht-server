#!/usr/bin/env bash
set -e

FOCLOIREACHT_DIR="$(cd "$(dirname "$0")" && pwd)"
FOCLOIREACHT_ROOT="$(cd "$FOCLOIREACHT_DIR/.." && pwd)"
GRAMADOIR_DIR="$(cd "$FOCLOIREACHT_DIR/../../gramadoir-server/mcp" && pwd)"
VENV="$FOCLOIREACHT_DIR/.venv"
CONFIG="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
DB_DIR="$FOCLOIREACHT_ROOT/vendor/irish-lex-db"

# --- Python venv + deps ---
echo "Creating venv..."
python3 -m venv "$VENV"
PYTHON="$VENV/bin/python"

echo "Installing Python deps..."
"$PYTHON" -m pip install -q -r "$FOCLOIREACHT_DIR/requirements.txt"
"$PYTHON" -m pip install -q -r "$GRAMADOIR_DIR/requirements.txt"

# --- Docker containers ---
echo "Starting focloireacht-server container..."
docker rm -f focloireacht 2>/dev/null || true
docker run -d \
  --name focloireacht \
  --restart unless-stopped \
  -p 5005:5005 \
  -v "$DB_DIR:/data:ro" \
  caffalaughrey/focloireacht-server:latest

echo "Starting gramadoir-server container..."
docker rm -f gramadoir 2>/dev/null || true
docker run -d \
  --name gramadoir \
  --restart unless-stopped \
  -p 5050:5000 \
  caffalaughrey/gramadoir:latest

# --- Claude Desktop config ---
echo "Updating Claude Desktop config..."
mkdir -p "$(dirname "$CONFIG")"
[ -f "$CONFIG" ] || echo '{}' > "$CONFIG"

"$PYTHON" - <<EOF
import json

config_path = """$CONFIG"""
python = """$PYTHON"""
focloireacht_script = """$FOCLOIREACHT_DIR/server.py"""
gramadoir_script = """$GRAMADOIR_DIR/server.py"""

with open(config_path) as f:
    config = json.load(f)

config.setdefault("mcpServers", {})

config["mcpServers"]["focloireacht"] = {
    "command": python,
    "args": [focloireacht_script],
    "env": {"FOCLOIREACHT_URL": "http://localhost:5005"}
}

config["mcpServers"]["gramadoir"] = {
    "command": python,
    "args": [gramadoir_script],
    "env": {"GRAMADOIR_URL": "http://localhost:5050"}
}

with open(config_path, "w") as f:
    json.dump(config, f, indent=2)

print("Done. Restart Claude Desktop to pick up the new MCPs.")
EOF
