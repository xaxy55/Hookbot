#!/bin/bash
# Xcode Cloud pre-build script
#
# Stamps the build number, and ensures automatic signing can resolve profiles.
#
# project.yml pins CURRENT_PROJECT_VERSION to 1, and build-testflight.sh
# overrides it at archive time with the git commit count. Xcode Cloud does not
# run that script, so without this every cloud build shipped as build 1 —
# App Store Connect rejects a build number it has already seen, so those builds
# could never reach TestFlight. Using the same commit-count scheme keeps the two
# build paths numbering consistently instead of racing each other.
set -uo pipefail

echo "--- ci_pre_xcodebuild: preparing for archive export ---"
echo "Scheme: ${CI_XCODE_SCHEME:-unknown}"
echo "Action: ${CI_XCODE_ACTION:-unknown}"

REPO="${CI_PRIMARY_REPOSITORY_PATH:-$(cd "$(dirname "$0")/../.." && pwd)}"
PBXPROJ="$REPO/ios/Hookbot.xcodeproj/project.pbxproj"

# A shallow clone makes the commit count meaningless, and silently wrong build
# numbers are worse than a slow fetch.
if [ "$(git -C "$REPO" rev-parse --is-shallow-repository 2>/dev/null)" = "true" ]; then
    echo "Repository is shallow; fetching full history for the build number..."
    git -C "$REPO" fetch --unshallow --quiet 2>/dev/null || \
        git -C "$REPO" fetch --depth=1000000 --quiet 2>/dev/null || true
fi

BUILD_NUMBER="$(git -C "$REPO" rev-list --count HEAD 2>/dev/null || echo 0)"
if [ -z "$BUILD_NUMBER" ] || [ "$BUILD_NUMBER" -lt 2 ]; then
    # Git could not tell us; fall back to Xcode Cloud's own counter rather than
    # shipping a 1 that will be rejected.
    BUILD_NUMBER="${CI_BUILD_NUMBER:-1}"
    echo "Falling back to CI_BUILD_NUMBER: $BUILD_NUMBER"
fi

echo "Build number: $BUILD_NUMBER"

if [ -f "$PBXPROJ" ]; then
    sed -i '' -E "s|CURRENT_PROJECT_VERSION = [^;]*;|CURRENT_PROJECT_VERSION = ${BUILD_NUMBER};|g" "$PBXPROJ"
    applied=$(grep -c "CURRENT_PROJECT_VERSION = ${BUILD_NUMBER};" "$PBXPROJ" || true)
    if [ "$applied" -gt 0 ]; then
        echo "Stamped $applied build configuration(s)"
    else
        # Failing loudly beats uploading a build that collides with an existing
        # number and disappears without explanation.
        echo "ERROR: could not stamp CURRENT_PROJECT_VERSION in $PBXPROJ" >&2
        exit 1
    fi
else
    echo "ERROR: $PBXPROJ not found" >&2
    exit 1
fi

echo "--- ci_pre_xcodebuild: done ---"
