#!/bin/bash
# Xcode Cloud ci_post_clone.sh
# Injects HOOKBOT_SERVER_URL from Xcode Cloud environment variables
# into the project build settings so $(HOOKBOT_SERVER_URL) resolves in Info.plist.

set -euo pipefail

echo "--- ci_post_clone: injecting build settings ---"

PBXPROJ="${CI_PRIMARY_REPOSITORY_PATH}/ios/Hookbot.xcodeproj/project.pbxproj"

if [ -n "${HOOKBOT_SERVER_URL:-}" ]; then
    echo "Setting HOOKBOT_SERVER_URL = ${HOOKBOT_SERVER_URL}"
    # Replace the default value in all build configurations
    sed -i '' "s|HOOKBOT_SERVER_URL = \"https://hookbot.mr-ai.no\"|HOOKBOT_SERVER_URL = \"${HOOKBOT_SERVER_URL}\"|g" "$PBXPROJ"
    echo "Updated project.pbxproj"
else
    echo "HOOKBOT_SERVER_URL not set, using default from build settings"
fi

echo "--- ci_post_clone: done ---"
