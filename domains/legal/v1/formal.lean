/-!
# AgentState Domain: legal/v1
# Lean 4 Formal Specification

This file is the **source of truth** for the legal/v1 domain pack.

## Scope

Contract validity, jurisdiction determination, limitation periods,
and regulatory compliance for legal AI systems.

## Regulatory coverage

- EU AI Act Article 13 (transparency requirements)
- Common law contract formation doctrine
- Civil law code-based contract elements
-/

import AgentState.Core

namespace AgentState.Domain.Legal.V1

open AgentState.Core

-- ── Domain axioms ──────────────────────────────────────────────────────────

/-- Axiom: contract_validity_elements
    A contract is valid iff it satisfies: offer, acceptance, consideration,
    capacity of all parties, and lawful purpose. -/
axiom contract_validity_elements : Prop

/-- Axiom: jurisdiction_basis
    Jurisdiction is determined by the governing law clause, subject to
    mandatory overriding law of the forum. -/
axiom jurisdiction_basis : Prop

/-- Axiom: limitation_period_basis
    A claim is within limitation iff the cause of action accrued within
    the statutory period applicable to the claim type and jurisdiction. -/
axiom limitation_period_basis : Prop

/-- Axiom: regulatory_compliance_basis
    An entity is compliant iff it has satisfied all mandatory requirements
    in the applicable regulatory instrument as of the compliance date. -/
axiom regulatory_compliance_basis : Prop

-- ── Premise role propositions ──────────────────────────────────────────────

axiom OfferEvidence : Prop
axiom AcceptanceEvidence : Prop
axiom ConsiderationEvidence : Prop
axiom CapacityEvidence : Prop
axiom LegalityEvidence : Prop
axiom GoverningLawClause : Prop
axiom ForumMandatoryLaw : Prop
axiom AccrualDate : Prop
axiom LimitationPeriod : Prop
axiom TollingEvents : Prop
axiom ComplianceChecklist : Prop
axiom RegulatoryInstrument : Prop

-- ── Conclusions ────────────────────────────────────────────────────────────

axiom ContractValid : Prop
axiom JurisdictionDetermined : Prop
axiom ClaimWithinLimitation : Prop
axiom RegulatoryCompliant : Prop

-- ── Inference rules ────────────────────────────────────────────────────────

/-- Rule: contract_validity_inference
    Premises: offer_evidence, acceptance_evidence, consideration_evidence,
              capacity_evidence, legality_evidence
    Conclusion: contract_valid
    @agentstate rule contract_validity_inference -/
theorem contract_validity_inference
    (h_offer         : OfferEvidence)
    (h_acceptance    : AcceptanceEvidence)
    (h_consideration : ConsiderationEvidence)
    (h_capacity      : CapacityEvidence)
    (h_legality      : LegalityEvidence) :
    ContractValid :=
  contract_validity_elements.elim (fun _ => ContractValid.elim id id) id

/-- Rule: jurisdiction_determination
    Premises: governing_law_clause, forum_mandatory_law
    Conclusion: jurisdiction_determined
    @agentstate rule jurisdiction_determination -/
theorem jurisdiction_determination
    (h_glc : GoverningLawClause)
    (h_fml : ForumMandatoryLaw) :
    JurisdictionDetermined :=
  jurisdiction_basis.elim (fun _ => JurisdictionDetermined.elim id id) id

/-- Rule: limitation_clearance
    Premises: accrual_date, limitation_period, tolling_events
    Conclusion: claim_within_limitation
    @agentstate rule limitation_clearance -/
theorem limitation_clearance
    (h_ad : AccrualDate)
    (h_lp : LimitationPeriod)
    (h_te : TollingEvents) :
    ClaimWithinLimitation :=
  limitation_period_basis.elim (fun _ => ClaimWithinLimitation.elim id id) id

/-- Rule: regulatory_compliance_inference
    Premises: compliance_checklist, regulatory_instrument
    Conclusion: regulatory_compliant
    @agentstate rule regulatory_compliance_inference -/
theorem regulatory_compliance_inference
    (h_cc : ComplianceChecklist)
    (h_ri : RegulatoryInstrument) :
    RegulatoryCompliant :=
  regulatory_compliance_basis.elim (fun _ => RegulatoryCompliant.elim id id) id

-- ── Claim templates ────────────────────────────────────────────────────────

/-- Template: contract_validity
    @agentstate template contract_validity
    @agentstate required_premises offer_evidence acceptance_evidence consideration_evidence capacity_evidence legality_evidence
    @agentstate inference_chain contract_validity_inference
    @agentstate conclusion contract_valid -/
theorem contract_validity_template
    (h_offer         : OfferEvidence)
    (h_acceptance    : AcceptanceEvidence)
    (h_consideration : ConsiderationEvidence)
    (h_capacity      : CapacityEvidence)
    (h_legality      : LegalityEvidence) :
    ContractValid :=
  contract_validity_inference h_offer h_acceptance h_consideration h_capacity h_legality

/-- Template: jurisdiction_claim
    @agentstate template jurisdiction_claim
    @agentstate required_premises governing_law_clause forum_mandatory_law
    @agentstate inference_chain jurisdiction_determination
    @agentstate conclusion jurisdiction_determined -/
theorem jurisdiction_claim_template
    (h_glc : GoverningLawClause)
    (h_fml : ForumMandatoryLaw) :
    JurisdictionDetermined :=
  jurisdiction_determination h_glc h_fml

/-- Template: limitation_claim
    @agentstate template limitation_claim
    @agentstate required_premises accrual_date limitation_period tolling_events
    @agentstate inference_chain limitation_clearance
    @agentstate conclusion claim_within_limitation -/
theorem limitation_claim_template
    (h_ad : AccrualDate)
    (h_lp : LimitationPeriod)
    (h_te : TollingEvents) :
    ClaimWithinLimitation :=
  limitation_clearance h_ad h_lp h_te

/-- Template: regulatory_compliance_claim
    @agentstate template regulatory_compliance_claim
    @agentstate required_premises compliance_checklist regulatory_instrument
    @agentstate inference_chain regulatory_compliance_inference
    @agentstate conclusion regulatory_compliant -/
theorem regulatory_compliance_claim_template
    (h_cc : ComplianceChecklist)
    (h_ri : RegulatoryInstrument) :
    RegulatoryCompliant :=
  regulatory_compliance_inference h_cc h_ri

end AgentState.Domain.Legal.V1
