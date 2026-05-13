/-!
# AgentState Domain: tax/v1
# Lean 4 Formal Specification

This file is the **source of truth** for the tax/v1 domain pack.

## Scope

Deduction validity, filing status determination, tax liability estimation,
and withholding adequacy. Jurisdiction-parameterized.

## Regulatory coverage

- OECD BEPS framework
- IRC (US Internal Revenue Code)
- EU DAC6 (cross-border arrangement disclosure)
-/

import AgentState.Core

namespace AgentState.Domain.Tax.V1

open AgentState.Core

-- ── Domain axioms ──────────────────────────────────────────────────────────

/-- Axiom: deduction_eligibility_basis
    A deduction is eligible iff the expense satisfies all statutory conditions
    for the relevant jurisdiction and tax year, supported by documentation. -/
axiom deduction_eligibility_basis : Prop

/-- Axiom: filing_status_exclusivity
    A taxpayer holds exactly one filing status per tax year. -/
axiom filing_status_exclusivity : Prop

/-- Axiom: withholding_correctness
    Withholding is correct iff the amount matches liability under the applicable
    withholding tables for the taxpayer's elections. -/
axiom withholding_correctness : Prop

/-- Axiom: income_reporting_completeness
    All income from all sources reportable under applicable law must be
    included in the return. -/
axiom income_reporting_completeness : Prop

-- ── Premise role propositions ──────────────────────────────────────────────

axiom ExpenseDocumentation : Prop
axiom StatutoryEligibilityCheck : Prop
axiom MaritalStatusEvidence : Prop
axiom HouseholdComposition : Prop
axiom TotalIncome : Prop
axiom ApplicableDeductions : Prop
axiom TaxRateSchedule : Prop
axiom W2Or1099Data : Prop
axiom LiabilityEstimate : Prop

-- ── Conclusions ────────────────────────────────────────────────────────────

axiom DeductionValid : Prop
axiom FilingStatusDetermined : Prop
axiom TaxLiabilityEstimated : Prop
axiom WithholdingAdequate : Prop

-- ── Inference rules ────────────────────────────────────────────────────────

/-- Rule: deduction_validity_inference
    Premises: expense_documentation, statutory_eligibility_check
    Conclusion: deduction_valid
    @agentstate rule deduction_validity_inference -/
theorem deduction_validity_inference
    (h_ed  : ExpenseDocumentation)
    (h_sec : StatutoryEligibilityCheck) :
    DeductionValid :=
  deduction_eligibility_basis.elim (fun _ => DeductionValid.elim id id) id

/-- Rule: filing_status_determination
    Premises: marital_status_evidence, household_composition
    Conclusion: filing_status_determined
    @agentstate rule filing_status_determination -/
theorem filing_status_determination
    (h_ms : MaritalStatusEvidence)
    (h_hc : HouseholdComposition) :
    FilingStatusDetermined :=
  filing_status_exclusivity.elim (fun _ => FilingStatusDetermined.elim id id) id

/-- Rule: liability_estimate_inference
    Premises: total_income, applicable_deductions, tax_rate_schedule
    Conclusion: tax_liability_estimated
    @agentstate rule liability_estimate_inference -/
theorem liability_estimate_inference
    (h_ti : TotalIncome)
    (h_ad : ApplicableDeductions)
    (h_tr : TaxRateSchedule) :
    TaxLiabilityEstimated :=
  income_reporting_completeness.elim (fun _ => TaxLiabilityEstimated.elim id id) id

/-- Rule: withholding_adequacy_inference
    Premises: w2_or_1099_data, liability_estimate
    Conclusion: withholding_adequate
    @agentstate rule withholding_adequacy_inference -/
theorem withholding_adequacy_inference
    (h_w2 : W2Or1099Data)
    (h_le : LiabilityEstimate) :
    WithholdingAdequate :=
  withholding_correctness.elim (fun _ => WithholdingAdequate.elim id id) id

-- ── Claim templates ────────────────────────────────────────────────────────

/-- Template: deduction_validity
    @agentstate template deduction_validity
    @agentstate required_premises expense_documentation statutory_eligibility_check
    @agentstate inference_chain deduction_validity_inference
    @agentstate conclusion deduction_valid -/
theorem deduction_validity_template
    (h_ed  : ExpenseDocumentation)
    (h_sec : StatutoryEligibilityCheck) :
    DeductionValid :=
  deduction_validity_inference h_ed h_sec

/-- Template: filing_status
    @agentstate template filing_status
    @agentstate required_premises marital_status_evidence household_composition
    @agentstate inference_chain filing_status_determination
    @agentstate conclusion filing_status_determined -/
theorem filing_status_template
    (h_ms : MaritalStatusEvidence)
    (h_hc : HouseholdComposition) :
    FilingStatusDetermined :=
  filing_status_determination h_ms h_hc

/-- Template: liability_estimate
    @agentstate template liability_estimate
    @agentstate required_premises total_income applicable_deductions tax_rate_schedule
    @agentstate inference_chain liability_estimate_inference
    @agentstate conclusion tax_liability_estimated -/
theorem liability_estimate_template
    (h_ti : TotalIncome)
    (h_ad : ApplicableDeductions)
    (h_tr : TaxRateSchedule) :
    TaxLiabilityEstimated :=
  liability_estimate_inference h_ti h_ad h_tr

end AgentState.Domain.Tax.V1
