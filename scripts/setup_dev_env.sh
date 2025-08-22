#!/usr/bin/env bash
set -euo pipefail

# Create a .env file with AGENTSTATE_URL and AGENTSTATE_API_KEY for local dev.

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

KID="${CAP_KEY_ACTIVE_ID:-active}"
SECRET="${CAP_KEY_ACTIVE:-dev-secret}"
URL="${AGENTSTATE_URL:-http://localhost:8080}"

echo "Generating dev token (kid=$KID) for namespaces: $*"

NS_ARGS=()
if [ "$#" -eq 0 ]; then
  NS_ARGS+=(--ns my-app)
else
  for ns in "$@"; do NS_ARGS+=(--ns "$ns"); done
fi

TOKEN=$(python3 scripts/generate_cap_token.py \
  --kid "$KID" \
  --secret "$SECRET" \
  ${NS_ARGS[@]} \
  --verb put --verb get --verb delete --verb query --verb lease)

cat > .env <<EOF
AGENTSTATE_URL=$URL
AGENTSTATE_API_KEY=$TOKEN
# Optional for LangChain demo
# OPENAI_API_KEY=sk-your-key
EOF

echo "Wrote .env with AGENTSTATE_URL and AGENTSTATE_API_KEY"
