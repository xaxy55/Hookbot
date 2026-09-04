#!/bin/bash
# Xcode Cloud ci_post_clone.sh
#
# Intentionally does nothing.
#
# It used to patch a HOOKBOT_SERVER_URL build setting into project.pbxproj so
# the app could ship with a default server baked in. That is gone: the app asks
# for the server at sign-in instead. Baking one in was wrong in both
# directions — the repo is public, so no hostname belongs in it, and a built-in
# default silently points an install at a server that isn't the user's, which
# is exactly how an app ends up talking to a host whose backend no longer
# exists.
#
# Kept as a no-op rather than deleted because Xcode Cloud runs this file by
# name if it is present; leaving a stub documents that nothing is expected here.
set -euo pipefail

echo "--- ci_post_clone: nothing to inject (server URL is entered in the app) ---"
