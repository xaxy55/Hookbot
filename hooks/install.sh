#!/usr/bin/env bash
# Install hookbot hooks into Claude Code settings
# Merges hook entries without overwriting existing hooks

set -euo pipefail

SETTINGS_FILE="$HOME/.claude/settings.json"
HOOK_SCRIPT="$(cd "$(dirname "$0")" && pwd)/hookbot-hook.js"

# Ensure hook script is executable
chmod +x "$HOOK_SCRIPT"

# Ensure settings file exists
if [ ! -f "$SETTINGS_FILE" ]; then
    mkdir -p "$(dirname "$SETTINGS_FILE")"
    echo '{}' > "$SETTINGS_FILE"
fi

# Check for jq
if ! command -v jq &>/dev/null; then
    echo "Error: jq is required. Install with: brew install jq"
    exit 1
fi

echo "Installing hookbot hooks..."
echo "Hook script: $HOOK_SCRIPT"
echo "Settings file: $SETTINGS_FILE"
echo ""

# Offer backup
read -r -p "Create backup of settings before modifying? [Y/n] " response
response=${response:-Y}
if [[ "$response" =~ ^[Yy]$ ]]; then
    BACKUP="$SETTINGS_FILE.bak.$(date +%Y%m%d%H%M%S)"
    cp "$SETTINGS_FILE" "$BACKUP"
    echo "Backup saved to: $BACKUP"
    echo ""
fi

# Build the hook entries to merge (new format: matcher + hooks array)
HOOK_ENTRIES=$(cat <<EOF
{
  "hooks": {
    "PreToolUse": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "node $HOOK_SCRIPT PreToolUse"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "node $HOOK_SCRIPT PostToolUse"
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "node $HOOK_SCRIPT UserPromptSubmit"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "node $HOOK_SCRIPT Stop"
          }
        ]
      }
    ]
  }
}
EOF
)

# Merge hooks into existing settings
# For each hook event, append our entry to the existing array (or create it)
MERGED=$(jq --argjson new "$HOOK_ENTRIES" '
  # Ensure .hooks exists
  .hooks //= {} |

  # For each hook event in the new entries, merge into existing
  reduce ($new.hooks | to_entries[]) as $entry (
    .;
    .hooks[$entry.key] = ((.hooks[$entry.key] // [])
      # Remove any existing hookbot entries to avoid duplicates
      | map(select(
          (.hooks // []) | all((.command // "") | contains("hookbot-hook") | not)
        ))
      # Append new entries
      + $entry.value)
  )
' "$SETTINGS_FILE")

# Write back
echo "$MERGED" | jq '.' > "$SETTINGS_FILE"

echo ""
echo "Hooks installed successfully!"
echo ""
echo "Registered hooks:"
jq -r '.hooks | to_entries[] | select(.value[] | .hooks[]? | select((.command // "") | contains("hookbot"))) | .key' "$SETTINGS_FILE" | sort -u
echo ""
echo "Make sure your ESP32 is running and reachable at the host in hookbot-config.json"
echo "Test with: curl -X POST http://localhost:3000/api/hook -H 'Content-Type: application/json' -d '{\"event\":\"PreToolUse\"}'"
