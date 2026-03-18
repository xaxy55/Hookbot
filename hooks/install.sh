#!/usr/bin/env bash
# Interactive installer for Hookbot Claude Code hooks
# Supports global install or per-project install with device selection

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
HOOK_SCRIPT="$SCRIPT_DIR/deskbot-hook.js"
GLOBAL_SETTINGS="$HOME/.claude/settings.json"
GLOBAL_CONFIG="$SCRIPT_DIR/deskbot-config.json"

# Colors
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BOLD='\033[1m'
NC='\033[0m'

echo ""
echo -e "${BOLD}╔══════════════════════════════════════╗${NC}"
echo -e "${BOLD}║       Hookbot Installer v0.2.0       ║${NC}"
echo -e "${BOLD}╚══════════════════════════════════════╝${NC}"
echo ""

# Ensure hook script is executable
chmod +x "$HOOK_SCRIPT"

# Check for jq
if ! command -v jq &>/dev/null; then
    echo -e "${RED}Error: jq is required. Install with: brew install jq${NC}"
    exit 1
fi

# --- Read server host from existing config ---
SERVER_HOST="http://localhost:3000"
if [ -f "$GLOBAL_CONFIG" ]; then
    SERVER_HOST=$(jq -r '.host // "http://localhost:3000"' "$GLOBAL_CONFIG")
fi

echo -e "${CYAN}Server:${NC} $SERVER_HOST"
read -r -p "Change server host? [y/N] " change_host
if [[ "$change_host" =~ ^[Yy]$ ]]; then
    read -r -p "Enter server host (e.g. http://localhost:3000): " SERVER_HOST
fi
echo ""

# --- Fetch devices from server ---
echo -e "${CYAN}Fetching devices from server...${NC}"
DEVICES_JSON=""
if DEVICES_JSON=$(curl -sf --connect-timeout 3 "$SERVER_HOST/api/devices" 2>/dev/null); then
    DEVICE_COUNT=$(echo "$DEVICES_JSON" | jq 'length')
    if [ "$DEVICE_COUNT" -gt 0 ]; then
        echo -e "${GREEN}Found $DEVICE_COUNT device(s):${NC}"
        echo ""
        echo "$DEVICES_JSON" | jq -r 'to_entries[] | "  \(.key + 1)) \(.value.name // "unnamed") (\(.value.device_type // "unknown")) - \(.value.ip_address // "no ip") [id: \(.value.id[0:8])...]"'
        echo ""
    else
        echo -e "${YELLOW}No devices registered on the server.${NC}"
        echo "Register your ESP devices via the web dashboard first, or enter a device ID manually."
        echo ""
    fi
else
    echo -e "${YELLOW}Could not reach server at $SERVER_HOST${NC}"
    echo "You can still install hooks and configure the device ID manually."
    echo ""
    DEVICES_JSON=""
    DEVICE_COUNT=0
fi

# --- Select device ---
DEVICE_ID=""
DEVICE_NAME=""

if [ -n "$DEVICES_JSON" ] && [ "$DEVICE_COUNT" -gt 0 ]; then
    read -r -p "Select device number (or 'm' for manual entry, Enter to skip): " device_choice

    if [[ "$device_choice" =~ ^[0-9]+$ ]] && [ "$device_choice" -ge 1 ] && [ "$device_choice" -le "$DEVICE_COUNT" ]; then
        IDX=$((device_choice - 1))
        DEVICE_ID=$(echo "$DEVICES_JSON" | jq -r ".[$IDX].id")
        DEVICE_NAME=$(echo "$DEVICES_JSON" | jq -r ".[$IDX].name // \"unnamed\"")
        echo -e "${GREEN}Selected: $DEVICE_NAME ($DEVICE_ID)${NC}"
    elif [[ "$device_choice" == "m" ]]; then
        read -r -p "Enter device ID: " DEVICE_ID
        DEVICE_NAME="manual"
    fi
else
    read -r -p "Enter device ID (or press Enter to skip): " DEVICE_ID
    if [ -n "$DEVICE_ID" ]; then
        DEVICE_NAME="manual"
    fi
fi
echo ""

# --- Global or Project ---
echo -e "${BOLD}Installation mode:${NC}"
echo "  1) Global  - hooks fire for all projects (default device)"
echo "  2) Project - hooks fire only in a specific project directory"
echo ""
read -r -p "Choose [1/2]: " install_mode

case "$install_mode" in
    2)
        INSTALL_TYPE="project"
        ;;
    *)
        INSTALL_TYPE="global"
        ;;
esac
echo ""

# --- Project path (if project mode) ---
PROJECT_PATH=""
if [ "$INSTALL_TYPE" = "project" ]; then
    read -r -e -p "Enter path to project directory: " PROJECT_PATH

    # Expand ~ to home dir
    PROJECT_PATH="${PROJECT_PATH/#\~/$HOME}"

    # Resolve to absolute path
    if [ -d "$PROJECT_PATH" ]; then
        PROJECT_PATH="$(cd "$PROJECT_PATH" && pwd)"
    else
        echo -e "${RED}Error: Directory does not exist: $PROJECT_PATH${NC}"
        exit 1
    fi

    echo -e "${CYAN}Project: $PROJECT_PATH${NC}"
    echo ""
fi

# --- Backup ---
if [ "$INSTALL_TYPE" = "global" ]; then
    TARGET_SETTINGS="$GLOBAL_SETTINGS"
else
    TARGET_SETTINGS="$PROJECT_PATH/.claude/settings.json"
fi

if [ -f "$TARGET_SETTINGS" ]; then
    read -r -p "Create backup of settings before modifying? [Y/n] " response
    response=${response:-Y}
    if [[ "$response" =~ ^[Yy]$ ]]; then
        BACKUP="$TARGET_SETTINGS.bak.$(date +%Y%m%d%H%M%S)"
        cp "$TARGET_SETTINGS" "$BACKUP"
        echo -e "Backup saved to: ${CYAN}$BACKUP${NC}"
        echo ""
    fi
fi

# --- Install hooks into settings ---
echo -e "${CYAN}Installing hooks...${NC}"

# Ensure target settings file exists
mkdir -p "$(dirname "$TARGET_SETTINGS")"
if [ ! -f "$TARGET_SETTINGS" ]; then
    echo '{}' > "$TARGET_SETTINGS"
fi

# Build the hook entries
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

# Merge hooks into settings (remove old hookbot/deskbot entries, add new ones)
MERGED=$(jq --argjson new "$HOOK_ENTRIES" '
  .hooks //= {} |
  reduce ($new.hooks | to_entries[]) as $entry (
    .;
    .hooks[$entry.key] = ((.hooks[$entry.key] // [])
      | map(select(
          (.hooks // []) | all((.command // "") | (contains("hookbot-hook") or contains("deskbot-hook")) | not)
        ))
      + $entry.value)
  )
' "$TARGET_SETTINGS")

echo "$MERGED" | jq '.' > "$TARGET_SETTINGS"

# --- Configure device binding ---
if [ "$INSTALL_TYPE" = "global" ]; then
    # Update global config
    CONFIG_CONTENT=$(jq -n \
        --arg host "$SERVER_HOST" \
        --arg mode "server" \
        --arg device_id "$DEVICE_ID" \
        'if $device_id == "" then {host: $host, mode: $mode} else {host: $host, mode: $mode, device_id: $device_id} end')
    echo "$CONFIG_CONTENT" | jq '.' > "$GLOBAL_CONFIG"
    echo -e "${GREEN}Global config updated:${NC} $GLOBAL_CONFIG"
else
    # Create per-project .hookbot config
    HOOKBOT_FILE="$PROJECT_PATH/.hookbot"
    CONFIG_CONTENT=$(jq -n \
        --arg host "$SERVER_HOST" \
        --arg mode "server" \
        --arg device_id "$DEVICE_ID" \
        'if $device_id == "" then {host: $host, mode: $mode} else {host: $host, mode: $mode, device_id: $device_id} end')
    echo "$CONFIG_CONTENT" | jq '.' > "$HOOKBOT_FILE"
    echo -e "${GREEN}Project config created:${NC} $HOOKBOT_FILE"
fi

# --- Summary ---
echo ""
echo -e "${BOLD}════════════════════════════════════════${NC}"
echo -e "${GREEN}Installation complete!${NC}"
echo ""
echo -e "  Mode:     ${CYAN}$INSTALL_TYPE${NC}"
if [ "$INSTALL_TYPE" = "project" ]; then
    echo -e "  Project:  ${CYAN}$PROJECT_PATH${NC}"
fi
echo -e "  Server:   ${CYAN}$SERVER_HOST${NC}"
if [ -n "$DEVICE_ID" ]; then
    echo -e "  Device:   ${CYAN}$DEVICE_NAME${NC} ($DEVICE_ID)"
else
    echo -e "  Device:   ${YELLOW}not set (will use first registered device)${NC}"
fi
echo -e "  Settings: ${CYAN}$TARGET_SETTINGS${NC}"
echo ""
echo -e "Registered hooks:"
jq -r '.hooks | to_entries[] | select(.value[] | .hooks[]? | select((.command // "") | contains("deskbot-hook"))) | "  ✓ \(.key)"' "$TARGET_SETTINGS" | sort -u
echo ""

if [ "$INSTALL_TYPE" = "project" ]; then
    echo -e "${YELLOW}Tip:${NC} Add .hookbot to your project's .gitignore"
fi
echo -e "Test with: ${CYAN}curl -X POST $SERVER_HOST/api/hook -H 'Content-Type: application/json' -d '{\"event\":\"PreToolUse\"}'${NC}"
echo ""
