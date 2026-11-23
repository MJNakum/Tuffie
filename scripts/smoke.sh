#!/usr/bin/env bash
# End-to-end smoke test. Uses tests/fixtures/mock-terraform, so no real
# Terraform or cloud account is needed.
set -euo pipefail
cd "$(dirname "$0")/.."
BIN=${BIN:-./target/debug/tuffie}
TMP=$(mktemp -d)
PORT=${PORT:-7791}
B="http://127.0.0.1:$PORT/api"
MOCK="$PWD/tests/fixtures/mock-terraform"
trap 'kill $PID 2>/dev/null || true; rm -rf "$TMP"' EXIT

mkdir -p "$TMP/ws" && cp examples/basic/main.tf "$TMP/ws/"
TUFFIE_LOG=warn "$BIN" serve --data-dir "$TMP/data" --listen "127.0.0.1:$PORT" --no-open &
PID=$!
for _ in $(seq 50); do curl -sf "$B/system/info" >/dev/null && break; sleep 0.1; done

json() { python3 -c "import json,sys; print(json.load(sys.stdin)$1)"; }
fail() { echo "FAIL: $*"; exit 1; }

WS=$(curl -sf -XPOST "$B/workspaces" -H 'content-type: application/json' \
  -d "{\"name\":\"smoke\",\"source_kind\":\"local\",\"local_path\":\"$TMP/ws\",\"tool_path\":\"$MOCK\"}" | json '["id"]')
curl -sf -XPOST "$B/workspaces/$WS/variables" -H 'content-type: application/json' \
  -d '{"key":"pw","value":"hunter2","category":"terraform","sensitive":true}' >/dev/null

RUN=$(curl -sf -XPOST "$B/workspaces/$WS/runs" -H 'content-type: application/json' -d '{"kind":"plan_and_apply"}' | json '["id"]')
for _ in $(seq 50); do [ "$(curl -sf "$B/runs/$RUN" | json '["status"]')" = planned ] && break; sleep 0.1; done
[ "$(curl -sf "$B/runs/$RUN" | json '["status"]')" = planned ] || fail "run did not reach planned"
curl -sf "$B/runs/$RUN/plan" | grep -q '"(sensitive value)"' || fail "sensitive plan value not masked"

curl -sf -XPOST "$B/runs/$RUN/apply" -o /dev/null
for _ in $(seq 50); do [ "$(curl -sf "$B/runs/$RUN" | json '["status"]')" = applied ] && break; sleep 0.1; done
[ "$(curl -sf "$B/runs/$RUN" | json '["status"]')" = applied ] || fail "run did not apply"
[ "$(curl -sf "$B/runs/$RUN" | json '["has_state_snapshot"]')" = True ] || fail "no state snapshot"

grep -rq hunter2 "$TMP/data/runs" && fail "plaintext secret found in run files"
[ "$(curl -s -o /dev/null -w '%{http_code}' -H 'Host: evil.example' "$B/system/info")" = 403 ] || fail "bad Host accepted"
[ "$(curl -s -o /dev/null -w '%{http_code}' -XPOST -H 'Origin: http://evil.example' "$B/workspaces" -d '{}')" = 403 ] || fail "cross-origin POST accepted"
[ "$(curl -s -o /dev/null -w '%{http_code}' "$B/workspaces/$WS/files/content?path=../../etc/passwd")" = 400 ] || fail "path traversal accepted"
[ "$(curl -s -o /dev/null -w '%{http_code}' "$B/workspaces/$WS/files/content?path=terraform.tfstate")" = 403 ] || fail "state file opened in editor"

echo "smoke test passed"
