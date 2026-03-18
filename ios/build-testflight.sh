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
ARCHIVE_PATH="$PROJECT_DIR/build/Hookbot.xcarchive"
EXPORT_PATH="$PROJECT_DIR/build/export"

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

# Step 2: Archive
echo "→ Archiving..."
xcodebuild archive \
    -project Hookbot.xcodeproj \
    -scheme "$SCHEME" \
    -destination "generic/platform=iOS" \
    -archivePath "$ARCHIVE_PATH" \
    -allowProvisioningUpdates \
    CODE_SIGN_STYLE=Automatic \
    | tail -1

echo "→ Archive created at $ARCHIVE_PATH"

# Step 3: Export for App Store / TestFlight
echo "→ Exporting for TestFlight..."
xcodebuild -exportArchive \
    -archivePath "$ARCHIVE_PATH" \
    -exportOptionsPlist "$PROJECT_DIR/ExportOptions.plist" \
    -exportPath "$EXPORT_PATH" \
    -allowProvisioningUpdates \
    | tail -1

echo "→ Export complete at $EXPORT_PATH"

# Step 4: Upload to App Store Connect
echo "→ Uploading to TestFlight..."
xcrun altool --upload-app \
    -f "$EXPORT_PATH/Hookbot.ipa" \
    -t ios \
    --apiKey "${APP_STORE_API_KEY:-}" \
    --apiIssuer "${APP_STORE_API_ISSUER:-}" \
    2>/dev/null || {
    echo ""
    echo "Auto-upload skipped. Upload manually via:"
    echo "  1. Xcode → Window → Organizer → Distribute App"
    echo "  2. Or: xcrun altool --upload-app -f $EXPORT_PATH/Hookbot.ipa -t ios --apiKey KEY --apiIssuer ISSUER"
    echo "  3. Or: Transporter.app (drag .ipa)"
}

echo ""
echo "=== Done ==="
