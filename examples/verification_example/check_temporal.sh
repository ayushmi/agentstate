#!/usr/bin/env bash
# Run temporal property verification against a WAL directory.
#
# Usage:
#   ./check_temporal.sh [WAL_DIR] [NAMESPACE]
#
# Example:
#   ./check_temporal.sh /data/wal production

set -euo pipefail

WAL_DIR="${1:-.}"
NS="${2:-verify-demo}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== AgentState Temporal Verification ==="
echo "WAL dir:   $WAL_DIR"
echo "Namespace: $NS"
echo

agentstate verify \
  --dir "$WAL_DIR" \
  --ns "$NS" \
  --property "$SCRIPT_DIR/props/no_unknown_status.ltl.json" \
  --property "$SCRIPT_DIR/props/liveness.ltl.json" \
  --output "$SCRIPT_DIR/verify-report.json" \
  --fail-on-violation

echo
echo "Report written to: $SCRIPT_DIR/verify-report.json"
