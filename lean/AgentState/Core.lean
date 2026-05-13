/-!
# AgentState Core — Lean 4 Foundation

This module defines the foundational types for the AgentState formal proof
system. Domain packs import this module to declare their inference rules.

## Trust Model

AgentState proofs have two layers of trust:

1. **External evidence** (premises): modeled as axioms. An axiom in Lean
   represents a trusted input — evidence verified by a process outside Lean
   (e.g. a published study, a WAL state object at a pinned commit, a
   domain-certified sensor reading). The axiom documents *what* is assumed.

2. **Domain logic** (inference rules): modeled as theorems. A theorem proves
   that if the premises are present, the conclusion necessarily follows by the
   domain's declared causal mechanism. Lean's kernel verifies this derivation.

The boundary between axioms and theorems is the domain's epistemic frontier:
everything inside is machine-verified; everything outside is attested by human
expertise and external processes.
-/

namespace AgentState.Core

/-- A proof role identifies the semantic function a piece of evidence plays
    in a claim. Roles are strings declared in the domain pack's inference rules. -/
def Role := String

/-- A predicate is the logical content of a claim's conclusion.
    Predicates are strings declared in the domain pack's claim templates. -/
def Predicate := String

/-- A proof step kind — mirrors the Rust `StepKind` enum. -/
inductive StepKind
  | ground      -- leaf: externally verified evidence
  | inference   -- internal: derived from prior steps by a domain rule
  | conclusion  -- root: the final proved conclusion

/-- A well-formed proof must satisfy the six formal properties.
    This structure is a Lean-level mirror of `ProofProperties` in Rust.
    When a proof certificate type-checks in Lean, the following hold:
    - verifiable: the proof term is well-typed (Lean's kernel guarantees this)
    - sound: every step is licensed by a declared theorem (not an axiom)
    All other properties are checked by the Rust engine before export. -/
structure ProofProperties where
  self_consistent        : Bool
  minimal                : Bool
  has_predictive_constraint : Bool
  verifiable             : Bool  -- always true when Lean accepts the file
  sound                  : Bool  -- true when rules are theorems, not axioms
  monotonic              : Bool
  machine_verified       : Bool  -- always true when Lean accepts the file

end AgentState.Core
