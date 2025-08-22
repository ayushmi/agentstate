#!/usr/bin/env python3
"""
Generate an AgentState capability token (kid.payload.sig) for local testing.

Usage examples:

  # Minimal (namespace + verbs), secret taken from CAP_KEY_ACTIVE env var
  python scripts/generate_cap_token.py \
    --kid active \
    --ns langchain-demo --ns integration-test \
    --verb put --verb get --verb delete --verb query --verb lease

  # Provide secret explicitly and longer expiry
  python scripts/generate_cap_token.py \
    --kid active \
    --secret dev-secret \
    --ns langchain-demo --ns integration-test \
    --verb put --verb get --verb delete --verb query --verb lease \
    --ttl 7200

Then export as:
  export AGENTSTATE_API_KEY=$(python scripts/generate_cap_token.py --kid active --secret dev-secret --ns langchain-demo --verb put --verb get --verb delete --verb query --verb lease)
"""

import argparse
import base64
import hashlib
import hmac
import json
import os
import sys
import time


def b64url_nopad(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).decode().rstrip("=")


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--kid", default=os.environ.get("CAP_KEY_ACTIVE_ID", "active"), help="key id: usually 'active' or 'next'")
    p.add_argument("--secret", default=os.environ.get("CAP_KEY_ACTIVE", ""), help="HMAC secret matching server CAP_KEY_ACTIVE")
    p.add_argument("--ns", action="append", dest="namespaces", default=[], help="Allowed namespace (repeatable)")
    p.add_argument("--verb", action="append", dest="verbs", default=[], help="Allowed verb (put|get|delete|query|lease|admin) repeatable")
    p.add_argument("--ttl", type=int, default=3600, help="Token lifetime seconds (default 3600)")
    p.add_argument("--jti", default=None, help="Optional token id (random if omitted)")
    p.add_argument("--region", default=None, help="Optional region pin (must match server REGION if set)")
    args = p.parse_args()

    if not args.secret:
        print("error: missing --secret (or CAP_KEY_ACTIVE env)", file=sys.stderr)
        sys.exit(2)

    now = int(time.time())
    claims = {
        "ns": args.namespaces or ["default"],
        "verbs": args.verbs or ["put", "get", "delete", "query", "lease"],
        "iat": now,
        "exp": now + args.ttl,
    }
    if args.jti:
        claims["jti"] = args.jti
    if args.region:
        claims["region"] = args.region

    payload = json.dumps(claims, separators=(",", ":")).encode()
    sig = hmac.new(args.secret.encode(), payload, hashlib.sha256).digest()

    token = f"{args.kid}.{b64url_nopad(payload)}.{b64url_nopad(sig)}"
    print(token)


if __name__ == "__main__":
    main()

