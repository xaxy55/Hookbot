#!/bin/bash
# Setup script for Hookbot iOS + Apple Watch app
# Generates the Xcode project from project.yml

set -e

cd "$(dirname "$0")"

echo "=== Hookbot iOS Setup ==="

# Check for xcodegen
if ! command -v xcodegen &> /dev/null; then
    echo "XcodeGen not found. Installing via Homebrew..."
    if command -v brew &> /dev/null; then
        brew install xcodegen
    else
        echo "ERROR: Homebrew not found. Install XcodeGen manually:"
        echo "  brew install xcodegen"
        echo "  OR: mint install yonaskolb/XcodeGen"
        exit 1
    fi
fi

echo "Generating Xcode project..."
xcodegen generate

echo ""
echo "=== Setup Complete ==="
echo "Open Hookbot.xcodeproj in Xcode"
echo ""
echo "Before building:"
echo "  1. Select your Development Team in project settings"
echo "  2. Set the iOS target to your device"
echo "  3. For Watch: pair your Apple Watch in Xcode"
echo ""
echo "The iOS app listens on port 8080 for state changes."
echo "Update your hookbot-hook.js to point to your iPhone's IP."
