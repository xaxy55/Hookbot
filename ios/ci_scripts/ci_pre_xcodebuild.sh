#!/bin/bash
# Xcode Cloud pre-build script
# Ensure automatic signing can resolve all profiles.
#
# Deliberately does NOT set the build number. Xcode Cloud assigns one itself
# (TestFlight shows 122, 123 ... 127 while CURRENT_PROJECT_VERSION stayed 1 in
# the project), so stamping it here fixes nothing and adds a way to corrupt the
# project: if the commit count came back empty — a shallow clone, or git not
# resolving the path — the substitution wrote "CURRENT_PROJECT_VERSION = ;".
# build-testflight.sh still sets it for local archives, where nothing else will.
echo "--- ci_pre_xcodebuild: preparing for archive export ---"
echo "Scheme: ${CI_XCODE_SCHEME:-unknown}"
echo "Action: ${CI_XCODE_ACTION:-unknown}"
echo "--- ci_pre_xcodebuild: done ---"
