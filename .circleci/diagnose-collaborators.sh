#!/usr/bin/env bash
# Prints what the GitHub App installation can do on this repository.
# Prints no token and no login.
set -euo pipefail

for tool in openssl jq curl base64; do
  command -v "$tool" >/dev/null || { echo "missing tool: $tool" >&2; exit 1; }
done
: "${PCU_APP_ID:?PCU_APP_ID is not set}"
: "${PCU_PRIVATE_KEY:?PCU_PRIVATE_KEY is not set}"
owner="${CIRCLE_PROJECT_USERNAME:?}"
repo="${CIRCLE_PROJECT_REPONAME:?}"
api="${GITHUB_API_URL:-https://api.github.com}"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# The key is stored as PEM, as base64 of the PEM, with literal \n, or with
# spaces or quotes in place of line breaks. Rebuild a PEM openssl accepts.
key="$PCU_PRIVATE_KEY"
case "$key" in
  *BEGIN*) ;;
  *) key="$(printf '%s' "$key" | tr -d '[:space:]' | base64 -d)" ||
       { echo "PCU_PRIVATE_KEY is neither PEM nor valid base64" >&2; exit 1; } ;;
esac
key="$(printf '%b' "$key")"
begin="$(printf '%s' "$key" | grep -oE -- '-----BEGIN [A-Z ]+-----' | head -n1 || true)"
end="$(printf '%s' "$key" | grep -oE -- '-----END [A-Z ]+-----' | head -n1 || true)"
if [ -z "$begin" ] || [ -z "$end" ]; then
  echo "PCU_PRIVATE_KEY has no PEM header and footer" >&2
  exit 1
fi
body="$(printf '%s' "$key" | sed -E 's/-----(BEGIN|END) [A-Z ]+-----//g' | tr -d "[:space:]\"'")"
key="$(printf '%s\n%s\n%s\n' "$begin" "$(printf '%s' "$body" | fold -w 64)" "$end")"

b64url() { openssl base64 -A | tr '+/' '-_' | tr -d '='; }
now="$(date +%s)"
header="$(printf '{"alg":"RS256","typ":"JWT"}' | b64url)"
payload="$(printf '{"iat":%d,"exp":%d,"iss":"%s"}' "$((now - 60))" "$((now + 540))" "$PCU_APP_ID" | b64url)"
signature="$(printf '%s.%s' "$header" "$payload" | openssl dgst -sha256 -sign <(printf '%s\n' "$key") | b64url)"
jwt="$header.$payload.$signature"

permission_headers() {
  grep -iE '^(HTTP/|x-accepted-github-permissions)' "$1" | tr -d '\r' || true
}

# $1 is the bearer token; it goes to curl on stdin so it never appears in argv.
request() {
  local bearer="$1"
  shift
  printf 'header = "Authorization: Bearer %s"\n' "$bearer" |
    curl -sS -K - -H "Accept: application/vnd.github+json" \
      -H "X-GitHub-Api-Version: 2022-11-28" "$@"
}
# Stops with the HTTP status when a response body is not JSON.
require_json() {
  jq -e . "$1" >/dev/null 2>&1 || {
    echo "$3: response is not JSON" >&2
    permission_headers "$2" >&2
    exit 1
  }
}

echo "--- installation on $owner/$repo"
request "$jwt" -D "$work/inst.hdr" -o "$work/inst.json" \
  "$api/repos/$owner/$repo/installation"
permission_headers "$work/inst.hdr"
require_json "$work/inst.json" "$work/inst.hdr" "installation"
id="$(jq -r '.id // empty' "$work/inst.json")"
if [ -z "$id" ]; then
  echo "no installation: $(jq -r '.message // "unknown"' "$work/inst.json")" >&2
  exit 1
fi
jq '{repository_selection, permissions}' "$work/inst.json"

echo "--- installation token"
request "$jwt" -X POST -D "$work/tok.hdr" -o "$work/tok.json" \
  "$api/app/installations/$id/access_tokens"
require_json "$work/tok.json" "$work/tok.hdr" "installation token"
token="$(jq -r '.token // empty' "$work/tok.json")"
if [ -z "$token" ]; then
  echo "no token: $(jq -r '.message // "unknown"' "$work/tok.json")" >&2
  exit 1
fi
jq '{repository_selection, permissions}' "$work/tok.json"

collaborators() {
  echo "--- collaborators: $1"
  request "$token" -D "$work/c.hdr" -o "$work/c.json" \
    "$api/repos/$owner/$repo/collaborators?per_page=100$2"
  permission_headers "$work/c.hdr"
  require_json "$work/c.json" "$work/c.hdr" "collaborators"
  jq -r 'if type == "array" then
      "count: \(length)\(if length == 100 then " (truncated, more pages exist)" else "" end), with push or admin: \(map(select(.permissions.push or .permissions.admin)) | length), roles: \(group_by(.role_name) | map("\(.[0].role_name)=\(length)") | join(" "))"
    else "message: \(.message // "unknown")" end' "$work/c.json"
}
collaborators "all (default)" ""
collaborators "direct" "&affiliation=direct"
collaborators "outside" "&affiliation=outside"
collaborators "push filter" "&permission=push"
