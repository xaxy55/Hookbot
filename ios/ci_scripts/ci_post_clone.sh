#!/bin/bash
# Xcode Cloud ci_post_clone.sh
# Injects HOOKBOT_SERVER_URL from Xcode Cloud environment variables
# into the project build settings so $(HOOKBOT_SERVER_URL) resolves in Info.plist.

set -euo pipefail

echo "--- ci_post_clone: injecting build settings ---"

PBXPROJ="${CI_PRIMARY_REPOSITORY_PATH}/ios/Hookbot.xcodeproj/project.pbxproj"

if [ -n "${HOOKBOT_SERVER_URL:-}" ]; then
    echo "Setting HOOKBOT_SERVER_URL = ${HOOKBOT_SERVER_URL}"
    # The setting ships empty (no hostname in a public repo), so match the
    # empty value rather than a literal domain. Anchoring on the setting name
    # keeps this working whatever the current value is.
    sed -i '' -E "s|HOOKBOT_SERVER_URL = \"[^\"]*\";|HOOKBOT_SERVER_URL = \"${HOOKBOT_SERVER_URL}\";|g" "$PBXPROJ"
    if grep -q "HOOKBOT_SERVER_URL = \"${HOOKBOT_SERVER_URL}\";" "$PBXPROJ"; then
        echo "Updated project.pbxproj"
    else
        # Failing loudly beats shipping a build that silently points nowhere.
        echo "ERROR: HOOKBOT_SERVER_URL build setting not found in $PBXPROJ" >&2
        exit 1
    fi
else
    echo "HOOKBOT_SERVER_URL not set; the app will ask the user for a server."
fi

echo "--- ci_post_clone: done ---"
