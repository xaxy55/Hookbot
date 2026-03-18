#!/bin/bash
set -euo pipefail

# Hookbot TestFlight Build & Upload Script
# Prerequisites:
#   - Xcode with valid Apple Developer account signed in
#   - DEVELOPMENT_TEAM set in project.yml (or passed via env)
#   - App icon added to Assets.xcassets/AppIcon.appiconset
#   - App created in App Store Connect (Bundle ID: com.mr-ai.hookbot, Apple ID: 6760749563)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR"
SCHEME="Hookbot"

echo "=== Hookbot TestFlight Build ==="
echo "Bundle ID: com.mr-ai.hookbot"
echo "Apple ID: 6760749563"
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

# Step 2: Archive & upload iOS
echo ""
echo "=== iOS ==="
IOS_ARCHIVE="$PROJECT_DIR/build/Hookbot-iOS.xcarchive"
IOS_EXPORT="$PROJECT_DIR/build/export-ios"

echo "→ Archiving iOS..."
xcodebuild archive \
    -project Hookbot.xcodeproj \
    -scheme "$SCHEME" \
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
xcrun altool --upload-app \
    -f "$IOS_EXPORT/Hookbot.ipa" \
    -t ios \
    --apiKey "${APP_STORE_API_KEY:-}" \
    --apiIssuer "${APP_STORE_API_ISSUER:-}" \
    2>/dev/null || {
    echo "  Auto-upload skipped. Upload manually via Xcode Organizer or Transporter."
}

# Step 3: Archive & upload Mac Catalyst
echo ""
echo "=== Mac Catalyst ==="
MAC_ARCHIVE="$PROJECT_DIR/build/Hookbot-Mac.xcarchive"
MAC_EXPORT="$PROJECT_DIR/build/export-mac"

echo "→ Archiving Mac Catalyst..."
xcodebuild archive \
    -project Hookbot.xcodeproj \
    -scheme "$SCHEME" \
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
xcrun altool --upload-app \
    -f "$MAC_EXPORT/Hookbot.pkg" \
    -t macos \
    --apiKey "${APP_STORE_API_KEY:-}" \
    --apiIssuer "${APP_STORE_API_ISSUER:-}" \
    2>/dev/null || {
    echo "  Auto-upload skipped. Upload manually via Xcode Organizer or Transporter."
}

echo ""
echo "=== Done ==="
echo "Archives:"
echo "  iOS: $IOS_ARCHIVE"
echo "  Mac: $MAC_ARCHIVE"
