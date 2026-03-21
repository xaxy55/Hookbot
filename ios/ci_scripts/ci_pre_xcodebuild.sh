#!/bin/bash
# Xcode Cloud pre-build script
# Ensure automatic signing can resolve all profiles
echo "--- ci_pre_xcodebuild: preparing for archive export ---"
echo "Scheme: ${CI_XCODE_SCHEME:-unknown}"
echo "Action: ${CI_XCODE_ACTION:-unknown}"
echo "--- ci_pre_xcodebuild: done ---"
