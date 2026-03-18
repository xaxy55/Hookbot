#!/bin/bash
set -euo pipefail

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

echo "=== Hookbot TestFlight Build ==="
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

upload() {
    local file="$1"
    local type="$2"
    xcrun altool --upload-app \
        -f "$file" \
        -t "$type" \
        --apiKey "${APP_STORE_API_KEY:-}" \
        --apiIssuer "${APP_STORE_API_ISSUER:-}" \
        2>/dev/null || {
        echo "  Auto-upload skipped. Upload manually via Xcode Organizer or Transporter."
    }
}

# Step 2: Archive & upload iOS
echo ""
echo "=== iOS ==="
IOS_ARCHIVE="$PROJECT_DIR/build/Hookbot-iOS.xcarchive"
IOS_EXPORT="$PROJECT_DIR/build/export-ios"

echo "→ Archiving iOS..."
xcodebuild archive \
    -project Hookbot.xcodeproj \
    -scheme Hookbot \
    -destination "generic/platform=iOS" \
    -archivePath "$IOS_ARCHIVE" \
    -allowProvisioningUpdates \
    CODE_SIGN_STYLE=Automatic \
    | tail -1

echo "→ Exporting iOS for TestFlight..."
xcodebuild -exportArchive \
    -archivePath "$IOS_ARCHIVE" \
    -exportOptionsPlist "$PROJECT_DIR/ExportOptions.plist" \
    -exportPath "$IOS_EXPORT" \
    -allowProvisioningUpdates \
    | tail -1

echo "→ Uploading iOS to TestFlight..."
upload "$IOS_EXPORT/Hookbot.ipa" ios

# Step 3: Archive & upload Mac Catalyst
echo ""
echo "=== Mac Catalyst ==="
MAC_ARCHIVE="$PROJECT_DIR/build/Hookbot-Mac.xcarchive"
MAC_EXPORT="$PROJECT_DIR/build/export-mac"

echo "→ Archiving Mac Catalyst..."
xcodebuild archive \
    -project Hookbot.xcodeproj \
    -scheme Hookbot \
    -destination "generic/platform=macOS,variant=Mac Catalyst" \
    -archivePath "$MAC_ARCHIVE" \
    -allowProvisioningUpdates \
    CODE_SIGN_STYLE=Automatic \
    | tail -1

echo "→ Exporting Mac for TestFlight..."
xcodebuild -exportArchive \
    -archivePath "$MAC_ARCHIVE" \
    -exportOptionsPlist "$PROJECT_DIR/ExportOptions.plist" \
    -exportPath "$MAC_EXPORT" \
    -allowProvisioningUpdates \
    | tail -1

echo "→ Uploading Mac to TestFlight..."
upload "$MAC_EXPORT/Hookbot.pkg" macos

# Step 4: Archive & upload watchOS
echo ""
echo "=== watchOS ==="
WATCH_ARCHIVE="$PROJECT_DIR/build/Hookbot-watchOS.xcarchive"
WATCH_EXPORT="$PROJECT_DIR/build/export-watchos"

echo "→ Archiving watchOS..."
xcodebuild archive \
    -project Hookbot.xcodeproj \
    -scheme HookbotWatch \
    -destination "generic/platform=watchOS" \
    -archivePath "$WATCH_ARCHIVE" \
    -allowProvisioningUpdates \
    CODE_SIGN_STYLE=Automatic \
    | tail -1

echo "→ Exporting watchOS for TestFlight..."
xcodebuild -exportArchive \
    -archivePath "$WATCH_ARCHIVE" \
    -exportOptionsPlist "$PROJECT_DIR/ExportOptions.plist" \
    -exportPath "$WATCH_EXPORT" \
    -allowProvisioningUpdates \
    | tail -1

echo "→ Uploading watchOS to TestFlight..."
upload "$WATCH_EXPORT/HookbotWatch.ipa" watchos

echo ""
echo "=== Done ==="
echo "Archives:"
echo "  iOS:     $IOS_ARCHIVE"
echo "  Mac:     $MAC_ARCHIVE"
echo "  watchOS: $WATCH_ARCHIVE"
