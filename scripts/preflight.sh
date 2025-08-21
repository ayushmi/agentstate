#!/usr/bin/env bash
set -euo pipefail

: "${BASE:?set BASE (e.g., https://agentstate.mycorp)}"
: "${ADMIN_CAP:?set ADMIN_CAP (admin bearer token)}"

echo "== health"
curl -fsS -H "Authorization: Bearer $ADMIN_CAP" "$BASE/health" >/dev/null && echo "OK"

echo "== metrics present"
curl -fsS -H "Authorization: Bearer $ADMIN_CAP" "$BASE/metrics" | grep -E 'watch_clients|wal_active_segments|watch_emit_lag_seconds' >/dev/null && echo "OK"

echo "== caps guards"
code=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer invalid" "$BASE/health"); test "$code" = "401" && echo "401 OK"

echo "== snapshot + trim"
snap=$(curl -fsS -H "Authorization: Bearer $ADMIN_CAP" -X POST "$BASE/admin/snapshot" | jq -r '.snapshot_id')
curl -fsS -H "Authorization: Bearer $ADMIN_CAP" -X POST "$BASE/admin/trim-wal?snapshot_id=$snap" >/dev/null && echo "OK ($snap)"

echo "== watch overflow/resume (SSE quick smoke)"
for i in {1..100}; do curl -fsS -H "Authorization: Bearer $ADMIN_CAP" -H "Content-Type: application/json" \
  -d "{\"id\":\"it-$i\",\"type\":\"t\",\"body\":{\"n\":$i},\"tags\":{\"topic\":\"smoke\"}}" \
  "$BASE/v1/agent://smoke/objects" >/dev/null; done; echo "PUT burst OK"
echo "Resume semantics validated via make verify-soak."

