#!/usr/bin/env python3
"""
Claim Verification Example — AgentState

Demonstrates submitting a claim, receiving a formal proof, and challenging it.

Usage:
    pip install agentstate requests
    python submit_claim.py
"""
import json
import sys
sys.path.insert(0, "../../sdk-py")
from agentstate import AgentStateClient

SERVER = "http://localhost:8080"
NS = "demo"

client = AgentStateClient(SERVER, NS)

# 1. Check the server is up
if not client.health():
    print("ERROR: AgentState server not running at", SERVER)
    sys.exit(1)

# 2. List available domain packs
domains = client.list_domains()
print("Available domain packs:")
for d in domains:
    print(f"  {d['domain']}/{d['version']}: {d.get('description', '')} "
          f"(templates: {', '.join(d.get('templates', []))})")
print()

# 3. Submit a drug-safety claim in the healthcare domain
print("Submitting drug-safety claim...")
result = client.submit_claim(
    ns=NS,
    domain="healthcare/v1",
    template="drug_safety",
    assertion={
        "predicate": "safe_to_prescribe",
        "subject": {"patient_id": "p-001"},
        "object": {"drug": "amoxicillin", "dose_mg": 500},
    },
    premises=[
        {
            "kind": "source",
            "id": "allergy-db-v3",
            "role": "allergy_clearance",
        },
        {
            "kind": "domain_axiom",
            "axiom_id": "drug_class_membership",
            "role": "drug_class",
        },
        {
            "kind": "domain_axiom",
            "axiom_id": "no_documented_interaction",
            "role": "no_known_interaction",
        },
    ],
    consequences=[
        {
            "predicate": {"field": "body.last_safety_check", "required": True},
            "check_after_hours": 24,
        }
    ],
    cause={"actor": "prescribing-agent", "note": "automated safety check"},
)

claim = result["claim"]
proof = result["proof"]

print(f"Claim ID:    {claim['id']}")
print(f"Proof ID:    {proof['proof_id']}")
print(f"Status:      {proof['status']}")
print(f"Confidence:  {proof['confidence']}")
print()

props = proof["properties"]
print("Formal property checks:")
for k, v in props.items():
    symbol = "✓" if v else "✗"
    print(f"  {symbol} {k}")
print()

print(f"Proof has {len(proof['steps'])} steps:")
for step in proof["steps"]:
    kind = step["kind"].ljust(12)
    print(f"  [{step['step_id']}] {kind} {step['conclusion_predicate']}")
print()

# 4. Challenge the proof (simulate a skeptic)
print("Challenging step 0 (allergy clearance source)...")
challenge = client.challenge_claim(
    ns=NS,
    claim_id=claim["id"],
    reason="The allergy database version is outdated — v4 supersedes v3.",
    challenged_step=0,
    counter_evidence=["allergy-db-v4", "clinical-record-2026-04-01"],
)
print(f"Challenge ID: {challenge['challenge_id']}")
print(f"Status:       {challenge['status']}")
print()

print("Full proof JSON:")
print(json.dumps(proof, indent=2, default=str))
