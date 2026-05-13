#!/usr/bin/env python3
"""
lean-codegen.py — AgentState Tier 3 codegen

Parses an AgentState formal.lean domain specification and generates the
corresponding manifest.json domain pack.

The formal.lean file is the source of truth. manifest.json is derived
from it and should not be edited by hand.

Usage:
    python scripts/lean-codegen.py domains/healthcare/v1/formal.lean
    python scripts/lean-codegen.py domains/  # process all domains

The script reads @agentstate annotations in doc comments:

    /-- Template: drug_safety
        @agentstate template drug_safety
        @agentstate required_premises allergy_history current_medications dosage_evidence
        @agentstate inference_chain allergy_clearance interaction_clearance prescribing_safety
        @agentstate conclusion safe_to_prescribe -/

    /-- Axiom: solvency_definition
        Statement text here...
        @agentstate axiom solvency_definition -/

    /-- Rule: allergy_clearance
        @agentstate rule allergy_clearance -/
"""

import json
import re
import sys
from pathlib import Path
from typing import Any


# ── Parsing ────────────────────────────────────────────────────────────────

def parse_formal_lean(path: Path) -> dict[str, Any]:
    """Parse a formal.lean file and return a domain pack dict."""
    source = path.read_text(encoding="utf-8")

    # Extract namespace
    ns_match = re.search(r"namespace\s+(AgentState\.Domain\.\S+)", source)
    lean_module = ns_match.group(1) if ns_match else ""

    # Extract domain + version from namespace: AgentState.Domain.Healthcare.V1
    domain, version = "", ""
    if lean_module:
        parts = lean_module.split(".")
        if len(parts) >= 4:
            domain = parts[2].lower()
            version = "v" + parts[3].lstrip("V").lower() if len(parts) > 3 else "v1"

    # Extract module docstring (/-! ... -/)
    doc_match = re.search(r"/-!(.*?)-/", source, re.DOTALL)
    description = ""
    if doc_match:
        raw = doc_match.group(1).strip()
        # First paragraph after the title line
        lines = raw.split("\n")
        desc_lines = []
        for line in lines[1:]:
            line = line.strip().lstrip("# ")
            if line.startswith("##"):
                break
            if line:
                desc_lines.append(line)
        description = " ".join(desc_lines).strip()

    axioms = _parse_axioms(source)
    inference_rules = _parse_rules(source)
    claim_templates = _parse_templates(source)
    consistency_constraints = _build_consistency_constraints(claim_templates)

    return {
        "domain": domain,
        "version": version,
        "description": description,
        "lean_module": lean_module,
        "axioms": axioms,
        "inference_rules": inference_rules,
        "claim_templates": claim_templates,
        "consistency_constraints": consistency_constraints,
    }


# ── Axiom extraction ───────────────────────────────────────────────────────

def _parse_axioms(source: str) -> list[dict]:
    """Extract domain axioms (not premise-role axioms) from doc comments."""
    axioms = []
    # Match doc comments followed by `axiom <Name> : Prop` (not premise role axioms)
    # Premise role axioms are identified by @agentstate annotations
    pattern = re.compile(
        r"/\*\*\s*(.*?)\s*\*\*/"  # /** ... */ — not used, skip
        r"|/--\s*(.*?)-/",
        re.DOTALL
    )
    # Use a simpler approach: find all /-- ... -/ blocks preceding `axiom`
    blocks = re.findall(r"/-(.*?)-/\s*\n\s*axiom\s+(\w+)\s*:", source, re.DOTALL)
    for doc, name in blocks:
        # Skip if this is a premise role axiom (no @agentstate annotation and short doc)
        doc_stripped = doc.strip().strip("-").strip()
        if "@agentstate" in doc_stripped:
            continue
        # Extract statement: everything after the first line that isn't empty
        lines = [l.strip().lstrip("/ ") for l in doc_stripped.split("\n") if l.strip().lstrip("/ ")]
        statement = " ".join(lines).strip()
        if statement and name[0].isupper():  # Skip lowercase (proposition names)
            continue
        if statement:
            axioms.append({"id": _to_snake(name), "statement": statement})

    # Better: extract from "-- ── Domain axioms" section
    axioms.clear()
    axiom_section = re.search(
        r"-- ── Domain axioms.*?-- ── Premise",
        source,
        re.DOTALL
    )
    if axiom_section:
        section = axiom_section.group(0)
        # Find /-- ... -/ blocks in this section
        doc_blocks = re.findall(r"/-(.*?)-/", section, re.DOTALL)
        for doc in doc_blocks:
            doc = doc.strip().strip("-").strip()
            lines = [l.strip().lstrip("/ ") for l in doc.split("\n") if l.strip().lstrip("/ ")]
            if not lines:
                continue
            # First line is the name (e.g. "Axiom: solvency_definition")
            first = lines[0]
            id_match = re.search(r"Axiom:\s*(\w+)", first)
            if not id_match:
                continue
            axiom_id = id_match.group(1)
            statement = " ".join(lines[1:]).strip()
            if statement:
                axioms.append({"id": axiom_id, "statement": statement})

    return axioms


# ── Rule extraction ────────────────────────────────────────────────────────

RULE_PATTERN = re.compile(
    r"/-(.*?)-/\s*\ntheorem\s+(\w+)",
    re.DOTALL
)

def _parse_rules(source: str) -> list[dict]:
    """Extract inference rules from @agentstate-annotated theorem doc comments."""
    rules = []
    for doc, theorem_name in RULE_PATTERN.findall(source):
        if "@agentstate rule" not in doc and "@agentstate template" not in doc:
            continue
        if "@agentstate template" in doc:
            continue  # templates are handled separately

        # Extract premises and conclusion from doc comment
        premises = _extract_annotation(doc, "required_premises")
        if not premises:
            # Fall back to parsing from "Premises:" line in doc
            m = re.search(r"Premises:\s*(.+)", doc)
            if m:
                premises = [p.strip() for p in m.group(1).split(",")]

        conclusion = _extract_annotation(doc, "conclusion", first_only=True)
        if not conclusion:
            m = re.search(r"Conclusion:\s*(\w+)", doc)
            conclusion = m.group(1) if m else _to_snake(theorem_name)

        rule_id = _extract_annotation(doc, "rule", first_only=True) or _to_snake(theorem_name)

        # Description from first non-annotation line
        desc_lines = [
            l.strip().lstrip("/ ")
            for l in doc.split("\n")
            if l.strip().lstrip("/ ") and not l.strip().lstrip("/ ").startswith("@agentstate")
            and not l.strip().lstrip("/ ").startswith("Premises:")
            and not l.strip().lstrip("/ ").startswith("Conclusion:")
        ]
        description = desc_lines[0] if desc_lines else rule_id

        rules.append({
            "id": rule_id,
            "premises": premises,
            "conclusion": conclusion,
            "description": description,
        })

    return rules


# ── Template extraction ────────────────────────────────────────────────────

def _parse_templates(source: str) -> list[dict]:
    """Extract claim templates from @agentstate template-annotated doc comments."""
    templates = []
    for doc, theorem_name in RULE_PATTERN.findall(source):
        if "@agentstate template" not in doc:
            continue

        template_id = _extract_annotation(doc, "template", first_only=True) or _to_snake(theorem_name)
        required = _extract_annotation(doc, "required_premises")
        inference_chain = _extract_annotation(doc, "inference_chain")
        conclusion = _extract_annotation(doc, "conclusion", first_only=True) or _to_snake(theorem_name)

        desc_lines = [
            l.strip().lstrip("/ ")
            for l in doc.split("\n")
            if l.strip().lstrip("/ ")
            and not l.strip().lstrip("/ ").startswith("@agentstate")
            and not l.strip().lstrip("/ ").startswith("Template:")
        ]
        description = desc_lines[0].strip() if desc_lines else template_id

        templates.append({
            "id": template_id,
            "description": description,
            "required_premises": required,
            "optional_premises": [],
            "conclusion_predicate": conclusion,
            "inference_chain": inference_chain,
        })

    return templates


# ── Consistency constraints ────────────────────────────────────────────────

def _build_consistency_constraints(templates: list[dict]) -> list[dict]:
    """Auto-generate consistency constraints: no two proofs with contradictory predicates."""
    constraints = []
    for tmpl in templates:
        pred = tmpl["conclusion_predicate"]
        contra = "not_" + pred if not pred.startswith("not_") else pred[4:]
        constraints.append({
            "id": f"no_dual_{pred}",
            "description": f"Cannot simultaneously hold {pred} and {contra} for the same subject.",
            "contradicts_predicate": contra,
        })
    return constraints


# ── Helpers ────────────────────────────────────────────────────────────────

def _extract_annotation(doc: str, key: str, first_only: bool = False) -> list[str] | str:
    """Extract value(s) from @agentstate <key> <value...> annotations."""
    pattern = re.compile(rf"@agentstate\s+{re.escape(key)}\s+(.*?)(?=@agentstate|\Z)", re.DOTALL)
    match = pattern.search(doc)
    if not match:
        return "" if first_only else []
    raw = match.group(1).strip()
    tokens = raw.split()
    if first_only:
        return tokens[0] if tokens else ""
    return tokens


def _to_snake(name: str) -> str:
    """Convert PascalCase or camelCase to snake_case."""
    s = re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()
    return s


# ── Main ───────────────────────────────────────────────────────────────────

def process_file(lean_path: Path) -> None:
    pack = parse_formal_lean(lean_path)
    out_path = lean_path.parent / "manifest.json"
    # Read existing manifest to preserve fields not in formal.lean
    existing = {}
    if out_path.exists():
        try:
            existing = json.loads(out_path.read_text())
        except json.JSONDecodeError:
            pass
    # Merge: formal.lean fields take precedence
    merged = {**existing, **pack}
    out_path.write_text(json.dumps(merged, indent=2) + "\n", encoding="utf-8")
    print(f"Generated {out_path}")
    print(f"  Templates: {len(pack['claim_templates'])}")
    print(f"  Rules:     {len(pack['inference_rules'])}")
    print(f"  Axioms:    {len(pack['axioms'])}")
    if not pack["claim_templates"] and not pack["inference_rules"]:
        print("  WARNING: no templates or rules extracted. Check @agentstate annotations.")


def main():
    if len(sys.argv) < 2:
        print("Usage: lean-codegen.py <formal.lean | domain-dir/>")
        sys.exit(1)

    target = Path(sys.argv[1])

    if target.is_file():
        process_file(target)
    elif target.is_dir():
        lean_files = list(target.rglob("formal.lean"))
        if not lean_files:
            print(f"No formal.lean files found under {target}")
            sys.exit(1)
        for f in sorted(lean_files):
            print(f"\nProcessing {f}")
            process_file(f)
    else:
        print(f"Error: {target} is not a file or directory")
        sys.exit(1)


if __name__ == "__main__":
    main()
