# Claim Verification Examples

These examples demonstrate AgentState's formal claim verification system.

## Prerequisites

Start the AgentState server:

```bash
docker run -p 8080:8080 ghcr.io/agentstate/agentstate:latest
```

## Python example

```bash
pip install requests
python submit_claim.py
```

This will:
1. List available domain packs (healthcare, finance, tax, legal)
2. Submit a `drug_safety` claim in the `healthcare/v1` domain
3. Print the formal proof with all six property checks
4. Submit a challenge against step 0

## CLI example

```bash
# List domain packs
agentstate domain --server http://localhost:8080

# Submit from JSON file
agentstate claim submit --ns demo --file claim.json

# Get the proof
agentstate claim proof --ns demo <claim-id>

# Challenge
agentstate claim challenge --ns demo <claim-id> \
  --reason "Balance sheet not independently audited" \
  --step 0 \
  --counter audited-balance-sheet-2026
```

## Domain pack format

See `../../domains/` for the four built-in packs. Contribute additional domains by following the same JSON schema and opening a pull request.
