#!/usr/bin/env bash
# Static secret / credential-leak audit.
#
# Two classes of bug, both of which have actually bitten this repo:
#   1. A secret committed to the tree. The repo is public.
#   2. A secret the server holds correctly but then serializes into an API
#      response. Spotify's access_token and refresh_token shipped to the
#      browser this way — stored fine, leaked on the way out.
#
# Exits non-zero if anything is found, so CI and `make security-audit` fail.
set -uo pipefail
cd "$(dirname "$0")/.."

RED=$'\033[31m'; GREEN=$'\033[32m'; YELLOW=$'\033[33m'; DIM=$'\033[2m'; RESET=$'\033[0m'
findings=0

ALLOWLIST="$(dirname "$0")/audit-allowlist.txt"
allowed=0

# A finding is suppressed only if audit-allowlist.txt documents why.
is_allowed() {
    [ -f "$ALLOWLIST" ] || return 1
    while IFS= read -r line; do
        case "$line" in ''|'#'*) continue;; esac
        pattern="${line%%#*}"
        pattern="$(printf '%s' "$pattern" | sed 's/[[:space:]]*$//')"
        [ -z "$pattern" ] && continue
        case "$1" in *"$pattern"*) return 0;; esac
    done < "$ALLOWLIST"
    return 1
}

report() {  # report <severity> <message>
    if is_allowed "$2"; then
        allowed=$((allowed + 1))
        printf '%s[allowed]%s %s\n' "$DIM" "$RESET" "$2"
        return
    fi
    findings=$((findings + 1))
    printf '%s[%s]%s %s\n' "$RED" "$1" "$RESET" "$2"
}

# Only audit tracked files: build output and node_modules are full of false
# positives and are not what ships in the public repo.
tracked() { git ls-files "$@" 2>/dev/null; }

printf '%s>> 1. Secrets committed to the tree%s\n' "$YELLOW" "$RESET"

# .env holds real credentials and must never be tracked.
for f in $(tracked '.env' '*/.env' 'deploy/.env'); do
    report CRITICAL ".env file is tracked by git: $f"
done

# High-confidence provider key formats. Deliberately narrow — a noisy audit
# that everyone learns to ignore is worse than no audit.
patterns=(
    'sk-[A-Za-z0-9]\{32,\}:OpenAI/Anthropic-style API key'
    'gh[pousr]_[A-Za-z0-9]\{36\}:GitHub token'
    'xox[baprs]-[A-Za-z0-9-]\{10,\}:Slack token'
    'AKIA[0-9A-Z]\{16\}:AWS access key id'
    '-----BEGIN [A-Z ]*PRIVATE KEY-----:Private key'
    'eyJ[A-Za-z0-9_-]\{20,\}\.eyJ[A-Za-z0-9_-]\{20,\}\.:JWT with a payload'
)
for entry in "${patterns[@]}"; do
    pat="${entry%%:*}"; desc="${entry#*:}"
    while IFS= read -r hit; do
        [ -z "$hit" ] && continue
        report CRITICAL "$desc in ${hit%%:*}"
    done < <(tracked | xargs -r grep -lI "$pat" 2>/dev/null | grep -v '^scripts/audit-secrets.sh$')
done

# A literal http(s) URL with an embedded password.
while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    report CRITICAL "URL with inline credentials in ${hit%%:*}"
done < <(tracked | xargs -r grep -lIE 'https?://[^/[:space:]]+:[^/@[:space:]]+@' 2>/dev/null | grep -v '^scripts/audit-secrets.sh$')

printf '%s>> 2. Credentials serialized into API responses%s\n' "$YELLOW" "$RESET"

# Any Rust struct that derives Serialize and has a secret-looking field must
# either skip it or be a request type (Deserialize only). This is the check
# that would have caught the Spotify leak.
while IFS= read -r file; do
    python3 - "$file" <<'PY'
import re, sys
path = sys.argv[1]
src = open(path, encoding='utf-8', errors='replace').read()

SECRET = re.compile(r'\b(access_token|refresh_token|api_key|client_secret|password|password_hash|token_hash|private_key|secret)\b')
# Fields that are safe by construction: previews, booleans, and column names.
SAFE = re.compile(r'(bool|token_preview|has_|_at\b)')

for m in re.finditer(r'((?:#\[[^\]]*\]\s*)+)pub struct (\w+)\s*\{(.*?)\n\}', src, re.S):
    attrs, name, body = m.group(1), m.group(2), m.group(3)
    if 'Serialize' not in attrs or re.search(r'\bSerialize\b', attrs) is None:
        continue
    for line in body.splitlines():
        stripped = line.strip()
        if not stripped.startswith('pub '):
            continue
        field = stripped.split(':')[0].replace('pub ', '').strip()
        if not SECRET.search(field) or SAFE.search(stripped):
            continue
        # Look back for a serde skip attribute on this field.
        idx = body.find(line)
        preceding = body[:idx].rstrip().splitlines()[-3:] if idx > 0 else []
        if any('skip_serializing' in p or 'skip)' in p for p in preceding):
            continue
        print(f"{path}: struct {name} serializes `{field}`")
PY
done < <(tracked 'server/src/*.rs' 'server/src/**/*.rs') > /tmp/hb_serialize_hits.txt 2>/dev/null

while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    report HIGH "$hit"
done < /tmp/hb_serialize_hits.txt
rm -f /tmp/hb_serialize_hits.txt

printf '%s>> 3. Secrets logged%s\n' "$YELLOW" "$RESET"

# Only flag a log line that interpolates a secret-named *argument*. Mentioning
# a variable's name in the message text ("Hashing admin password from ...") is
# not a leak, and flagging it trains people to ignore the audit.
while IFS= read -r hit; do
    [ -z "$hit" ] && continue
    report MEDIUM "secret interpolated into a log line: $hit"
done < <(tracked 'server/src/*.rs' 'server/src/**/*.rs' \
    | xargs -r grep -nE '(info|warn|error|debug|println)!\([^)]*\{\}[^)]*",[^)]*\b(access_token|refresh_token|api_key|client_secret|password)\b' 2>/dev/null \
    | cut -c1-160)

echo
if [ "$findings" -eq 0 ]; then
    printf '%s>> No credential leaks found.%s' "$GREEN" "$RESET"
    [ "$allowed" -gt 0 ] && printf '%s (%d allowlisted)%s' "$DIM" "$allowed" "$RESET"
    printf '\n'
    exit 0
fi
printf '%s>> %d finding(s).%s\n' "$RED" "$findings" "$RESET"
printf '%s   A tracked secret must be rotated, not just deleted — it is in git history.%s\n' "$DIM" "$RESET"
exit 1
