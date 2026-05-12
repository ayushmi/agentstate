# We built open-source formal verification for AI agents — because autonomous operations demand proof, not logging

We are building AI that runs operations in the physical world. Dispatching setpoint changes to PLCs. Executing in biological systems. Coordinating the kind of high-consequence decisions where a wrong action is not a bad recommendation — it is a catastrophe.

That work forced us to confront a problem no existing tool solved: **how do you know an AI system did the right thing, for the right reason, at the right time — and prove it to someone who has every reason to be skeptical?**

Today we are open-sourcing [AgentState](https://github.com/ayushmi/agentstate) v1.2.0, which adds formal claim verification to an already verifiable agent state store. The whole thing — state management, hash chains, runtime invariants, temporal property checking, and now a full proof engine with domain packs for healthcare, finance, tax, and legal — is Apache-2.0.

This post is about why we built it, why we believe verifiability has to be open-source, and what we think it means for the field.

---

## The problem with "AI in the loop"

Most AI deployment today sits in an advisory position. A model recommends, a human decides, and if the model is wrong, you retrain it. The feedback loop is slow but the blast radius is manageable.

Autonomous operations are different. When AI dispatches an action to a PLC — a valve position, a temperature setpoint, a dosing rate — there is no human confirmation step. The action happens. In biology, in industrial processes, in logistics, the system is live and consequential.

The standard toolkit for this is monitoring and logging. Record what the system did. Alert on anomalies. Review incidents post-hoc. This is necessary but not sufficient. **Logging tells you what happened. It does not tell you whether what happened was justified.**

There is a deeper issue. Most AI systems reason by correlation — "historically, when conditions looked like X, action Y produced good outcomes." That is fine for recommendation. For autonomous operation it is dangerous, because the system cannot distinguish between "Y works because of X" and "Y happened to correlate with X in training data." When you encounter novel conditions, correlation-based reasoning extrapolates blindly. Mechanism-based reasoning — understanding *why* X causes Y — generalizes correctly and knows when it does not know.

We are building a causal world model for mechanism inference. The goal is AI that understands the mechanisms of the systems it operates, not just their historical patterns. But even a causal model needs an external verification layer. The causal understanding the model has must be expressible, inspectable, and challengeable by people who are not AI researchers — operators, regulators, domain experts.

That is the gap we built AgentState to fill.

---

## What we built

AgentState is an agent state store — think Redis with a WAL, a watch stream, and a query layer, designed specifically for AI agent workloads. That part is table stakes.

What differentiates it is four layers of formal verifiability, each composing on the previous.

### Layer 1: Tamper-evident hash chain

Every write extends a blake3 hash chain. Each commit includes the hash of the previous commit and a monotonic sequence number. Reordering, replacing, or deleting any WAL record breaks the chain and is immediately detectable.

This gives you a cryptographic audit trail of every belief update and every action dispatch. If something goes wrong, you know exactly what the system believed, in what order, and that the record has not been altered.

### Layer 2: Runtime invariant assertions

Before any write is accepted, it is validated against a predicate spec for the namespace. Violations are rejected with a structured error before they are stored.

```json
{
  "rules": [
    { "field": "body.pressure_bar", "lte": 12.0 },
    { "field": "body.status", "one_of": ["running", "standby", "shutdown"] },
    { "field": "body.operator_confirmed", "required": true }
  ]
}
```

This is not schema validation. It is a formal contract that no write can violate, enforced at the storage layer, WAL-persisted so it survives restarts.

### Layer 3: Temporal property checking (LTL)

Write formal properties about system behavior as JSON files. Run them against the full WAL execution trace — offline, in CI, or post-incident.

```json
{
  "name": "shutdown_always_precedes_restart",
  "kind": "safety",
  "forall": { "type": "process_unit" },
  "always": {
    "leads_to": {
      "if":   { "field": "body.status", "eq": "restarting" },
      "then": { "previously": { "field": "body.status", "eq": "shutdown" } }
    }
  }
}
```

```bash
agentstate-cli verify --dir /data/wal --ns plant-floor \
  --property props/shutdown_sequence.ltl.json \
  --fail-on-violation
```

This runs in CI. If the system ever violated the property across its entire operational history, the check fails with a counterexample trace.

### Layer 4: Claim verification

This is the new piece in v1.2.0, and the one we are most excited about.

When an AI system takes a consequential action, it has a reason. The causal model has a belief about why this action is warranted. Claim verification lets you make that belief explicit, formal, and provable.

A claim is a structured assertion: a predicate, a subject, an object, a set of premises (evidence that grounds the claim), and a set of consequences (testable things that must hold afterward). Submit a claim and you receive a **proof artifact** — a full DAG with step-by-step reasoning, hash-chained into the WAL.

```python
result = client.submit_claim(
    ns="plant-floor",
    domain="process-control/v1",
    assertion={
        "predicate": "safe_to_increase_setpoint",
        "subject":   {"unit": "reactor-3"},
        "object":    {"parameter": "temperature_c", "new_value": 87.5},
    },
    premises=[
        {"kind": "wal_state",    "object_id": "reactor-3-sensors",
         "field": "body.coolant_flow_nominal", "role": "cooling_verified",
         "at_commit": "a1b2c3d4"},
        {"kind": "source",       "id": "maintenance-log-2026-05",
         "role": "no_recent_faults"},
        {"kind": "domain_axiom", "axiom_id": "thermal_capacity_margin",
         "role": "thermal_headroom"},
    ],
    consequences=[
        {"predicate": {"field": "body.temperature_c", "lte": 90.0},
         "check_after_hours": 1}
    ],
)
```

Every proof is checked against six formal properties:

| Property | What it guarantees |
|---|---|
| Self-consistency | No contradicting proved claim exists in the namespace |
| Minimality | Every declared premise is load-bearing — no padding |
| Predictive constraint | At least one testable consequence is declared |
| Verifiability | The proof DAG is complete and fully replayable |
| Soundness | Every inference step is licensed by a declared domain rule |
| Monotonicity | WAL state premises are pinned to a specific commit, not floating |

If any property fails, the proof status is `refuted`. The system cannot silently produce a "proved" proof on bad foundations.

Anyone can challenge any proof — a domain expert, an operator, a regulator — by citing a specific step and counter-evidence. Challenges are themselves WAL-persisted.

---

## Domain packs

The proof engine works against formal domain packs — JSON manifests that define the axioms, inference rules, and valid claim templates for a domain. We ship four:

- **healthcare/v1** — drug safety, contraindication, diagnosis, lab normals
- **finance/v1** — solvency, capital adequacy (Basel III), AML clearance, collateral
- **tax/v1** — deduction validity, filing status, liability estimate
- **legal/v1** — contract validity, jurisdiction, limitation, regulatory compliance

These are community infrastructure. The schema is open. If you can formalize a domain — process control, agriculture, clinical trials, cybersecurity — you can contribute it as a JSON manifest and it becomes available to everyone using the system.

This is important. The formal knowledge required to verify AI actions in a specific domain (pharmaceutical manufacturing, grid operations, surgical robotics) is held by domain experts, not AI engineers. The contribution model has to make it possible for those experts to participate without writing Rust.

---

## Why open-source

The honest answer is that we had no choice.

We are deploying AI into high-consequence physical systems. Our customers — operators, regulators, safety engineers — need to trust the verification layer at least as much as they need to trust the AI itself. Trust in infrastructure of this kind cannot be built on a closed system. It requires source you can read, behavior you can audit, and a community that can find the bugs you missed.

There is also a second-order argument. Formal verification for AI is a young field. The right abstractions — what a "claim" should look like, what properties a proof should satisfy, how domain knowledge should be encoded — are not settled. The only way to converge on good answers is in the open, with practitioners across industries contributing what they learn.

Proprietary verification is not just a trust problem. It is an epistemological problem. If the verification system is closed, you cannot know whether the proofs it produces are actually sound. You are trusting the vendor's implementation of soundness, not soundness itself.

We want a world where the verification layer for autonomous AI is as open and scrutinizable as the Linux kernel. It should be infrastructure, not a product moat.

---

## What we are working toward

AgentState is the verifiable substrate. The larger project is building AI that can run operations — physical and digital — where the actions are causal and the reasoning is provable.

Autonomous operation in the physical world requires a model that understands mechanisms, not just patterns. When that model recommends a setpoint change, it should be able to produce a proof: here are the causal premises I am relying on, here is the inference chain, here is the consequence I predict, here is what would falsify this claim. That proof should be inspectable by the operator, reviewable by the regulator, and challengeable by anyone with relevant domain knowledge.

We are working on this in industrial operations and in biology. Biology in particular is a domain where the mechanism-correlation gap is enormous — biological systems are complex, interventions have unexpected consequences, and the cost of getting it wrong is high. Getting causal mechanism inference right in biology is one of the hardest and most important problems we know of.

AgentState is the open infrastructure layer. The causal world model and the autonomous action system sit on top of it. We are building those too, but the substrate needs to be open — for trust, for community, and because the problems are too large for any one team.

---

## Try it

```bash
docker run -p 8080:8080 ayushmi/agentstate:latest
pip install agentstate
```

Submit a claim against the healthcare domain:

```python
from agentstate import AgentStateClient

client = AgentStateClient("http://localhost:8080", "demo")

result = client.submit_claim(
    ns="demo",
    domain="healthcare/v1",
    template="drug_safety",
    assertion={
        "predicate": "safe_to_prescribe",
        "subject": {"patient_id": "p-001"},
        "object":  {"drug": "amoxicillin"},
    },
    premises=[
        {"kind": "source", "id": "allergy-db-v3", "role": "allergy_clearance"},
        {"kind": "domain_axiom", "axiom_id": "drug_class_membership", "role": "drug_class"},
    ],
    consequences=[
        {"predicate": {"field": "body.last_safety_check", "required": True}}
    ],
)

print(result["proof"]["status"])      # "proved"
print(result["proof"]["properties"])  # all six formal properties
```

**GitHub**: https://github.com/ayushmi/agentstate
**Docs**: https://github.com/ayushmi/agentstate/blob/main/docs/claim-verification.md

If you are working on autonomous operations, industrial AI, or biological systems, we want to talk. If you can formalize a domain, open a pull request. If you find something wrong with the proof engine, file an issue — that is exactly the kind of scrutiny this infrastructure needs.
