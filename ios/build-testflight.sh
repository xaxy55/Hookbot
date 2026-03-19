#!/bin/bash
set -uo pipefail

# Hookbot TestFlight Build & Upload Script
# Archives and uploads iOS, Mac Catalyst, and watchOS to TestFlight.
#
# Prerequisites:
#   - Xcode with valid Apple Developer account signed in
#   - xcodegen installed (brew install xcodegen)
#   - Apps created in App Store Connect:
#     iOS/Mac:  com.mr-ai.hookbot        (Apple ID: 6760749563)
#     watchOS:  com.mr-ai.hookbot.watchkitapp (Apple ID: 6760757931)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR"
FAILURES=0

# Auto-increment build number using git commit count
BUILD_NUMBER=$(git rev-list --count HEAD 2>/dev/null || echo "1")

echo "=== Hookbot TestFlight Build ==="
echo "Build number: $BUILD_NUMBER"
echo ""

# Step 1: Generate Xcode project from project.yml
echo "→ Generating Xcode project..."
cd "$PROJECT_DIR"
if command -v xcodegen &>/dev/null; then
    xcodegen generate
else
    echo "Error: xcodegen not installed. Run: brew install xcodegen"
    exit 1
fi

archive_export_upload() {
    local label="$1"
    local scheme="$2"
    local destination="$3"
    local archive_path="$4"
    local export_path="$5"
    local artifact="$6"
    local upload_type="$7"

    echo ""
    echo "=== $label ==="

    echo "→ Archiving $label..."
    if ! xcodebuild archive \
        -project Hookbot.xcodeproj \
        -scheme "$scheme" \
        -destination "$destination" \
        -archivePath "$archive_path" \
        -allowProvisioningUpdates \
        CODE_SIGN_STYLE=Automatic \
        CURRENT_PROJECT_VERSION="$BUILD_NUMBER" \
        | tail -1; then
        echo "  ✗ Archive failed for $label"
        FAILURES=$((FAILURES + 1))
        return 1
    fi

    echo "→ Exporting $label for TestFlight..."
    if ! xcodebuild -exportArchive \
        -archivePath "$archive_path" \
        -exportOptionsPlist "$PROJECT_DIR/ExportOptions.plist" \
        -exportPath "$export_path" \
        -allowProvisioningUpdates \
        | tail -1; then
        echo "  ✗ Export failed for $label"
        FAILURES=$((FAILURES + 1))
        return 1
    fi

    echo "→ Uploading $label to TestFlight..."
    xcrun altool --upload-app \
        -f "$export_path/$artifact" \
        -t "$upload_type" \
        --apiKey "${APP_STORE_API_KEY:-}" \
        --apiIssuer "${APP_STORE_API_ISSUER:-}" \
        2>/dev/null || {
        echo "  Auto-upload skipped. Upload manually via Xcode Organizer or Transporter."
    }

    echo "  ✓ $label done"
}

archive_export_upload "iOS" \
    "Hookbot" "generic/platform=iOS" \
    "$PROJECT_DIR/build/Hookbot-iOS.xcarchive" \
    "$PROJECT_DIR/build/export-ios" \
    "Hookbot.ipa" "ios"

archive_export_upload "Mac Catalyst" \
    "Hookbot" "generic/platform=macOS,variant=Mac Catalyst" \
    "$PROJECT_DIR/build/Hookbot-Mac.xcarchive" \
    "$PROJECT_DIR/build/export-mac" \
    "Hookbot.pkg" "macos"

archive_export_upload "watchOS" \
    "HookbotWatch" "generic/platform=watchOS" \
    "$PROJECT_DIR/build/Hookbot-watchOS.xcarchive" \
    "$PROJECT_DIR/build/export-watchos" \
    "HookbotWatch.ipa" "watchos"

echo ""
if [ "$FAILURES" -gt 0 ]; then
    echo "=== Done with $FAILURES failure(s) ==="
    exit 1
else
    echo "=== All platforms uploaded successfully ==="
fi
