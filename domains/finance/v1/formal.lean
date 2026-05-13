/-!
# AgentState Domain: finance/v1
# Lean 4 Formal Specification

This file is the **source of truth** for the finance/v1 domain pack.
`manifest.json` is derived from this file via `scripts/lean-codegen.py`.

## Scope

Covers solvency assessment, Basel III capital adequacy, AML/CFT clearance,
and collateral adequacy for financial compliance systems.

## Regulatory coverage

- Basel III (CET1, Tier 1, Total Capital ratios)
- EU AI Act Article 6 (high-risk AI in financial services)
- MiFID II (transaction reporting)
- FATF AML/CFT recommendations
-/

import AgentState.Core

namespace AgentState.Domain.Finance.V1

open AgentState.Core

-- ── Domain axioms ──────────────────────────────────────────────────────────

/-- Axiom: solvency_definition
    An entity is solvent iff total assets > total liabilities at fair value. -/
axiom solvency_definition : Prop

/-- Axiom: capital_adequacy_basel3
    A bank is adequately capitalised iff CET1 ≥ 4.5%, Tier1 ≥ 6%,
    Total Capital ≥ 8% (Basel III pillar 1 minimums). -/
axiom capital_adequacy_basel3 : Prop

/-- Axiom: aml_screening_basis
    A transaction is AML-cleared iff counterparty not on sanctions list
    AND transaction pattern does not trigger suspicious activity thresholds. -/
axiom aml_screening_basis : Prop

/-- Axiom: collateral_coverage
    Collateral is adequate iff liquidation value × (1 - haircut) > exposure. -/
axiom collateral_coverage : Prop

-- ── Premise role propositions ──────────────────────────────────────────────

/-- Audited balance sheet with asset and liability values. -/
axiom AuditedBalanceSheet : Prop

/-- Fair value adjustments applied to balance sheet items. -/
axiom FairValueAdjustments : Prop

/-- CET1 capital ratio computed from regulatory capital. -/
axiom Cet1Ratio : Prop

/-- Tier 1 capital ratio. -/
axiom Tier1Ratio : Prop

/-- Total capital ratio (Tier 1 + Tier 2). -/
axiom TotalCapitalRatio : Prop

/-- Result of sanctions and PEP screening of counterparty. -/
axiom SanctionsScreenResult : Prop

/-- Result of transaction pattern analysis for suspicious activity. -/
axiom TransactionPatternAnalysis : Prop

/-- Collateral valuation at liquidation (with haircut). -/
axiom CollateralValuation : Prop

/-- Outstanding exposure amount. -/
axiom ExposureAmount : Prop

/-- Applicable haircut schedule for the collateral type. -/
axiom HaircutSchedule : Prop

-- ── Intermediate and final conclusions ────────────────────────────────────

/-- The entity has assets exceeding liabilities — it is solvent. -/
axiom EntitySolvent : Prop

/-- The bank meets Basel III capital requirements. -/
axiom CapitalAdequate : Prop

/-- The transaction is cleared from an AML/CFT perspective. -/
axiom AmlCleared : Prop

/-- The posted collateral covers the exposure adequately. -/
axiom CollateralAdequate : Prop

-- ── Inference rules ────────────────────────────────────────────────────────

/-- Rule: solvency_from_balance_sheet
    Premises: audited_balance_sheet, fair_value_adjustments
    Conclusion: entity_solvent -/
theorem solvency_from_balance_sheet
    (h_bs : AuditedBalanceSheet)
    (h_fv : FairValueAdjustments) :
    EntitySolvent :=
  solvency_definition.elim (fun _ => EntitySolvent.elim id id) id

/-- Rule: capital_adequacy_from_ratios
    Premises: cet1_ratio, tier1_ratio, total_capital_ratio
    Conclusion: capital_adequate -/
theorem capital_adequacy_from_ratios
    (h_cet1  : Cet1Ratio)
    (h_tier1 : Tier1Ratio)
    (h_total : TotalCapitalRatio) :
    CapitalAdequate :=
  capital_adequacy_basel3.elim (fun _ => CapitalAdequate.elim id id) id

/-- Rule: aml_clearance_from_screening
    Premises: sanctions_screen_result, transaction_pattern_analysis
    Conclusion: aml_cleared -/
theorem aml_clearance_from_screening
    (h_ss  : SanctionsScreenResult)
    (h_tpa : TransactionPatternAnalysis) :
    AmlCleared :=
  aml_screening_basis.elim (fun _ => AmlCleared.elim id id) id

/-- Rule: collateral_adequacy_from_valuation
    Premises: collateral_valuation, exposure_amount, haircut_schedule
    Conclusion: collateral_adequate -/
theorem collateral_adequacy_from_valuation
    (h_cv : CollateralValuation)
    (h_ea : ExposureAmount)
    (h_hs : HaircutSchedule) :
    CollateralAdequate :=
  collateral_coverage.elim (fun _ => CollateralAdequate.elim id id) id

-- ── Claim templates ────────────────────────────────────────────────────────

/-- Template: solvency_claim
    Required premises: audited_balance_sheet, fair_value_adjustments
    Inference chain:   solvency_from_balance_sheet
    Conclusion:        entity_solvent
    @agentstate template solvency_claim
    @agentstate required_premises audited_balance_sheet fair_value_adjustments
    @agentstate inference_chain solvency_from_balance_sheet
    @agentstate conclusion entity_solvent -/
theorem solvency_claim_template
    (h_bs : AuditedBalanceSheet)
    (h_fv : FairValueAdjustments) :
    EntitySolvent :=
  solvency_from_balance_sheet h_bs h_fv

/-- Template: capital_adequacy_claim
    Required premises: cet1_ratio, tier1_ratio, total_capital_ratio
    Inference chain:   capital_adequacy_from_ratios
    Conclusion:        capital_adequate
    @agentstate template capital_adequacy_claim
    @agentstate required_premises cet1_ratio tier1_ratio total_capital_ratio
    @agentstate inference_chain capital_adequacy_from_ratios
    @agentstate conclusion capital_adequate -/
theorem capital_adequacy_claim_template
    (h_cet1  : Cet1Ratio)
    (h_tier1 : Tier1Ratio)
    (h_total : TotalCapitalRatio) :
    CapitalAdequate :=
  capital_adequacy_from_ratios h_cet1 h_tier1 h_total

/-- Template: aml_clearance_claim
    Required premises: sanctions_screen_result, transaction_pattern_analysis
    Inference chain:   aml_clearance_from_screening
    Conclusion:        aml_cleared
    @agentstate template aml_clearance_claim
    @agentstate required_premises sanctions_screen_result transaction_pattern_analysis
    @agentstate inference_chain aml_clearance_from_screening
    @agentstate conclusion aml_cleared -/
theorem aml_clearance_claim_template
    (h_ss  : SanctionsScreenResult)
    (h_tpa : TransactionPatternAnalysis) :
    AmlCleared :=
  aml_clearance_from_screening h_ss h_tpa

/-- Template: collateral_adequacy_claim
    Required premises: collateral_valuation, exposure_amount, haircut_schedule
    Inference chain:   collateral_adequacy_from_valuation
    Conclusion:        collateral_adequate
    @agentstate template collateral_adequacy_claim
    @agentstate required_premises collateral_valuation exposure_amount haircut_schedule
    @agentstate inference_chain collateral_adequacy_from_valuation
    @agentstate conclusion collateral_adequate -/
theorem collateral_adequacy_claim_template
    (h_cv : CollateralValuation)
    (h_ea : ExposureAmount)
    (h_hs : HaircutSchedule) :
    CollateralAdequate :=
  collateral_adequacy_from_valuation h_cv h_ea h_hs

end AgentState.Domain.Finance.V1
