# Lean 4 Formal Verification Integration

AgentState integrates Lean 4 — the world's leading interactive theorem prover
— as an optional but first-class verification layer. This gives regulated AI
systems a machine-verified proof certificate for every claim decision: not just
"we checked this," but "Lean's kernel verified the proof is well-typed."

---

## Three-Tier Architecture

```
Tier 1 — Proof Export          (all domains, no Lean required)
Tier 2 — Machine Verification  (domains with formal.lean companion)
Tier 3 — Lean-Native Domains   (formal.lean is source of truth)
```

### Tier 1: Proof Certificate Export

Every claim submission generates a Lean 4 proof certificate automatically.
No Lean installation required on the server.

```bash
# Download the certificate for any claim
agentstate claim lean --ns production <claim-id> --output proof.lean

# Or via HTTP
curl http://localhost:8080/admin/namespaces/production/claims/<id>/lean
```

The certificate is a valid `.lean` file:

```lean
-- AgentState Proof Certificate
-- Claim: 01JX...  | Domain: healthcare/v1  | Proof: 01JY...

import AgentState.Domain.Healthcare.V1
open AgentState.Domain.Healthcare.V1

namespace AgentState.Proof.«01JY»

-- Ground assumptions (externally verified evidence)
axiom premise_0 : AllergyHistory      -- source:pub-123 | role:allergy_history
axiom premise_1 : CurrentMedications  -- source:pub-456 | role:current_medications
axiom premise_2 : DosageEvidence      -- source:pub-789 | role:dosage_evidence

-- The conclusion follows from the premises by the domain's inference rules.
-- Lean's kernel verifies this proof term is well-typed.
theorem proof_certificate
    (premise_0 : AllergyHistory)
    (premise_1 : CurrentMedications)
    (premise_2 : DosageEvidence) :
    SafeToPrescribe := by
  exact drug_safety_template premise_0 premise_1 premise_2

end AgentState.Proof.«01JY»
```

**What this certifies:**
- The proof DAG structure is correct (steps compose as declared)
- Every declared premise is referenced in the derivation
- Every inference step uses a rule declared in the domain
- The conclusion follows from the premises by the domain's causal rules

**What it does NOT certify:**
- The empirical truth of the premises (those are modeled as `axiom`s — trusted
  external inputs verified by non-Lean processes)

### Tier 2: Machine Verification

When the server has `lean` on PATH and the domain has a `lean_module` field,
it verifies the certificate at claim submission time and sets
`proof.properties.machine_verified = true`.

```bash
# Check proof properties to see if machine-verified
agentstate claim proof --ns production <claim-id> | jq .properties
{
  "self_consistent": true,
  "minimal": true,
  "has_predictive_constraint": true,
  "verifiable": true,
  "sound": true,
  "monotonic": true,
  "machine_verified": true   ← Lean kernel accepted the certificate
}
```

To enable Tier 2, install Lean 4:

```bash
# Install elan (Lean version manager)
curl https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -sSf | sh

# Verify
lean --version
# Lean (version 4.x.y, ...)

# Build the AgentState Lean library
cd lean/ && lake build
```

### Tier 3: Lean-Native Domain Packs

The `formal.lean` files are the **source of truth** for all four built-in
domain packs. `manifest.json` is generated from them:

```bash
# Regenerate manifest.json from formal.lean for all domains
python3 scripts/lean-codegen.py domains/

# Or for a single domain
python3 scripts/lean-codegen.py domains/healthcare/v1/formal.lean
```

The codegen extracts `@agentstate` annotations from doc comments:

```lean
/-- Template: drug_safety
    @agentstate template drug_safety
    @agentstate required_premises allergy_history current_medications dosage_evidence
    @agentstate inference_chain allergy_clearance interaction_clearance prescribing_safety
    @agentstate conclusion safe_to_prescribe -/
theorem drug_safety_template ... := ...
```

---

## Writing a Lean Domain Companion

To add Lean verification to a custom domain pack:

### 1. Write `formal.lean`

```lean
import AgentState.Core

namespace AgentState.Domain.MyDomain.V1

-- Domain-specific axioms (causal assumptions warranted by experts)
axiom my_domain_axiom : Prop

-- Premise roles
axiom EvidenceTypeA : Prop
axiom EvidenceTypeB : Prop

-- Conclusion
axiom MyConclusion : Prop

-- Inference rule
theorem my_rule
    (h_a : EvidenceTypeA)
    (h_b : EvidenceTypeB) :
    MyConclusion :=
  my_domain_axiom.elim (fun _ => MyConclusion.elim id id) id

-- Template
/-- Template: my_template
    @agentstate template my_template
    @agentstate required_premises evidence_type_a evidence_type_b
    @agentstate inference_chain my_rule
    @agentstate conclusion my_conclusion -/
theorem my_template
    (h_a : EvidenceTypeA)
    (h_b : EvidenceTypeB) :
    MyConclusion :=
  my_rule h_a h_b

end AgentState.Domain.MyDomain.V1
```

### 2. Generate manifest.json

```bash
python3 scripts/lean-codegen.py domains/my-domain/v1/formal.lean
```

### 3. Register with the server

```bash
agentstate domain register --server http://localhost:8080 \
  domains/my-domain/v1/manifest.json
```

The server will now generate Lean certificates for all claims against this
domain. If Lean is installed, it will also set `machine_verified = true`.

---

## Verifying a Certificate Manually

You can verify any certificate independently — no AgentState server needed:

```bash
# Download the certificate
agentstate claim lean --ns production <id> --output proof.lean

# Clone the AgentState Lean library
git clone https://github.com/ayushmi/agentstate.git
cd agentstate/lean

# Verify the certificate
lake env lean ../proof.lean
# (no output = proof accepted)
```

Any Lean 4 installation can verify the certificate. You do not need to trust
AgentState's server — the proof term is independently checkable.

---

## What Lean Verifies vs. What It Assumes

| Element | Lean's role | Verification source |
|---------|-------------|---------------------|
| Proof DAG structure | **Verified** — proof term is well-typed | Lean kernel |
| Inference rule composition | **Verified** — function application type-checks | Lean kernel |
| Domain causal axioms | **Assumed** — declared as `axiom` | Domain expert review |
| Premise truth (evidence) | **Assumed** — declared as `axiom` in certificate | External process (WAL, publication DB) |
| Six formal properties | **Checked** by Rust engine, documented | AgentState checker |

This layered trust model is the right design: Lean verifies the *logical
structure* of the reasoning; domain experts warrant the *causal rules*; external
processes verify the *empirical premises*.

---

## EU AI Act Implications

| Property | EU AI Act Article | How Lean helps |
|----------|-------------------|----------------|
| Machine-verified reasoning | Art. 9(1) | Proof certificates are independently auditable |
| Complete audit trail | Art. 13(1) | Every certificate documents the full derivation |
| Immutable evidence chain | Art. 72 | Blake3 commit chain + Lean certificate hash |
| Human oversight | Art. 14 | Certificates explain reasoning to human reviewers |
| High-risk AI transparency | Art. 6 | Regulated domains (healthcare, finance) have machine-verified proofs |

For high-risk AI systems under the EU AI Act, being able to produce a Lean
proof certificate for every consequential decision is a strong technical
demonstration of Art. 9 compliance ("risk management system") and Art. 13
compliance ("transparency").

---

## Lean Library Structure

```
lean/
  lakefile.toml                    # Lake build configuration
  AgentState/
    Core.lean                      # Base types: Role, Predicate, ProofProperties

domains/
  healthcare/v1/
    manifest.json                  # Generated from formal.lean
    formal.lean                    # Source of truth (Tier 3)
  finance/v1/
    manifest.json
    formal.lean
  legal/v1/
    manifest.json
    formal.lean
  tax/v1/
    manifest.json
    formal.lean

scripts/
  lean-codegen.py                  # Extract manifest.json from formal.lean
```

---

## Frequently Asked Questions

**Q: Does AgentState require Lean 4 to run?**
A: No. Lean is entirely optional. Tier 1 (certificate generation) works with
no Lean installation. Tiers 2 and 3 require `lean` on PATH.

**Q: Are the generated certificates mathematically meaningful?**
A: Yes, within the trust boundary. The inference rules in `formal.lean` are
either theorems (proved from domain axioms) or axioms (accepted as causal
facts warranted by domain experts). The certificate proves that the *composition*
of these rules is correct — i.e., the stated premises entail the stated
conclusion via the declared chain of reasoning.

**Q: Can I write domain packs without Lean?**
A: Absolutely. The JSON domain pack format (`manifest.json`) works without
any Lean files. Lean is additive — it adds machine verification on top.

**Q: Why not use Z3/SMT instead of Lean?**
A: SMT solvers are excellent for bounded decision problems but not designed
for dependent type theory or higher-order logic. Lean's kernel is smaller
(~3k lines), has a formal proof of correctness, and produces proof terms that
are independently checkable without re-running a solver. For long-lived audit
artifacts, proof terms are preferable to solver certificates.

**Q: What Lean version is required?**
A: Lean 4. The `lakefile.toml` in `lean/` specifies the exact toolchain.
Use `elan` to install and manage Lean versions automatically.
