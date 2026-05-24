#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="pre-usage"
INSTALL_DIR="$HOME/.claude/plugins"
SETTINGS="$HOME/.claude/settings.json"
BINARY_PATH="$INSTALL_DIR/$BINARY"

echo "→ Building $BINARY (release)..."
cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

mkdir -p "$INSTALL_DIR"
cp "$SCRIPT_DIR/target/release/$BINARY" "$BINARY_PATH"
echo "→ Installed to $BINARY_PATH"

# Register (or update) the UserPromptSubmit hook in settings.json
if [[ ! -f "$SETTINGS" ]]; then
    echo "{}" > "$SETTINGS"
fi

python3 - <<PYEOF
import json, os, sys

settings_path = "$SETTINGS"
binary_path = "$BINARY_PATH"

with open(settings_path) as f:
    settings = json.load(f)

hooks = settings.setdefault("hooks", {})
ups = hooks.setdefault("UserPromptSubmit", [])

hook_entry = {"type": "command", "command": binary_path}
group_entry = {"hooks": [hook_entry]}

# Check if already registered
already = any(
    any(h.get("command") == binary_path for h in e.get("hooks", []))
    for e in ups
)

if already:
    print("→ UserPromptSubmit hook already registered in settings.json")
else:
    ups.append(group_entry)
    with open(settings_path, "w") as f:
        json.dump(settings, f, indent=2)
    print("→ Registered UserPromptSubmit hook in settings.json")
PYEOF

# Git pre-push hook — blocks direct pushes to main/master
GIT_DIR="$(git -C "$SCRIPT_DIR" rev-parse --git-dir 2>/dev/null || true)"
if [[ -n "$GIT_DIR" ]]; then
    ln -sf "$SCRIPT_DIR/hooks/pre-push" "$GIT_DIR/hooks/pre-push"
    echo "→ Installed git pre-push hook"
else
    echo "→ Not a git repo — skipping git hook install"
fi

echo ""
echo "✓ Done. Token check runs before every Claude prompt."
echo ""
echo "  Threshold (default 50K tokens):"
echo "    export PRE_USAGE_THRESHOLD=50K    # plain integer, or K / M suffix (default)"
echo "    export PRE_USAGE_THRESHOLD=100K"
echo "    export PRE_USAGE_THRESHOLD=1M"
echo ""
echo "  Strategy when threshold is exceeded (default: warn):"
echo "    export PRE_USAGE_STRATEGY=warn    # print warning and auto-proceed (default)"
echo "    export PRE_USAGE_STRATEGY=block   # require explicit confirmation"
echo ""
echo "  To remove: rm $BINARY_PATH"
echo "             then delete the UserPromptSubmit entry from $SETTINGS"
