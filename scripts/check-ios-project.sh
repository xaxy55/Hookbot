#!/usr/bin/env bash
# Verify every iOS source file is actually referenced by the Xcode project.
#
# project.yml is the source of truth, but Xcode Cloud builds the *committed*
# .xcodeproj and never runs xcodegen. So adding a .swift file without
# regenerating leaves it out of the build: locally everything still compiles
# because the typechecker is given the files directly, and CI fails on a symbol
# that "obviously exists". Cheap to check, so check it.
set -uo pipefail
cd "$(dirname "$0")/.."

RED=$'\033[31m'; GREEN=$'\033[32m'; DIM=$'\033[2m'; RESET=$'\033[0m'
PBXPROJ="ios/Hookbot.xcodeproj/project.pbxproj"

if [ ! -f "$PBXPROJ" ]; then
    printf '%s>> %s not found.%s\n' "$RED" "$PBXPROJ" "$RESET"
    exit 1
fi

# Excluded in project.yml, so their absence is correct.
EXCLUDED='GameScene.swift|GameViewController.swift|AppDelegate.swift'

missing=0
while IFS= read -r file; do
    base="$(basename "$file")"
    case "$base" in
        *.swift) ;;
        *) continue;;
    esac
    printf '%s' "$base" | grep -qE "^($EXCLUDED)$" && continue
    if ! grep -q "$base" "$PBXPROJ"; then
        printf '%s[missing]%s %s\n' "$RED" "$RESET" "$file"
        missing=$((missing + 1))
    fi
done < <(git ls-files 'ios/Hookbot/*.swift' 'ios/Shared/*.swift' \
                     'ios/HookbotWatch/*.swift' 'ios/HookbotiOSWidget/*.swift' 2>/dev/null)

if [ "$missing" -gt 0 ]; then
    printf '\n%s>> %d source file(s) are not in the Xcode project.%s\n' "$RED" "$missing" "$RESET"
    printf '%s   Xcode Cloud builds the committed project, so these will not compile there.%s\n' "$DIM" "$RESET"
    printf '%s   Fix with: cd ios && xcodegen generate%s\n' "$DIM" "$RESET"
    exit 1
fi

printf '%s>> Every iOS source file is in the Xcode project.%s\n' "$GREEN" "$RESET"
