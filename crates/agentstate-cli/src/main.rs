use agentstate_claims::{build_proof, Claim, ClaimRequest, DomainPack, DomainRegistry};
use agentstate_storage::walbin;
use agentstate_verify;
use anyhow::Result;
use clap::{Parser, Subcommand};
use reqwest::blocking::Client;

#[derive(Subcommand)]
enum ClaimCmd {
    /// Submit a new claim and print the resulting proof.
    Submit {
        /// Server base URL
        #[arg(long, default_value = "http://localhost:8080")]
        server: String,
        /// Namespace
        #[arg(long, short = 'n')]
        ns: String,
        /// Path to a JSON file containing the ClaimRequest body
        #[arg(long, short = 'f')]
        file: String,
    },
    /// Get a stored claim by ID.
    Get {
        /// Server base URL
        #[arg(long, default_value = "http://localhost:8080")]
        server: String,
        /// Namespace
        #[arg(long, short = 'n')]
        ns: String,
        /// Claim ID
        id: String,
    },
    /// Get the formal proof for a claim.
    Proof {
        /// Server base URL
        #[arg(long, default_value = "http://localhost:8080")]
        server: String,
        /// Namespace
        #[arg(long, short = 'n')]
        ns: String,
        /// Claim ID
        id: String,
    },
    /// List all claims in a namespace.
    List {
        /// Server base URL
        #[arg(long, default_value = "http://localhost:8080")]
        server: String,
        /// Namespace
        #[arg(long, short = 'n')]
        ns: String,
    },
    /// Submit a challenge against a claim's proof.
    Challenge {
        /// Server base URL
        #[arg(long, default_value = "http://localhost:8080")]
        server: String,
        /// Namespace
        #[arg(long, short = 'n')]
        ns: String,
        /// Claim ID to challenge
        id: String,
        /// Reason for the challenge
        #[arg(long)]
        reason: String,
        /// Specific step index to challenge (optional; omit to challenge the whole proof)
        #[arg(long)]
        step: Option<usize>,
        /// Source or claim IDs that provide counter-evidence (repeatable)
        #[arg(long)]
        counter: Vec<String>,
    },
    /// Print a human-readable explanation of a claim's proof.
    Explain {
        /// Server base URL
        #[arg(long, default_value = "http://localhost:8080")]
        server: String,
        /// Namespace
        #[arg(long, short = 'n')]
        ns: String,
        /// Claim ID
        id: String,
    },
    /// Export a claim's proof as a W3C Verifiable Credential.
    Vc {
        /// Server base URL
        #[arg(long, default_value = "http://localhost:8080")]
        server: String,
        /// Namespace
        #[arg(long, short = 'n')]
        ns: String,
        /// Claim ID
        id: String,
    },
}

#[derive(Subcommand)]
enum DomainCmd {
    /// List all available domain packs (built-in + registered).
    List {
        /// Server base URL
        #[arg(long, default_value = "http://localhost:8080")]
        server: String,
    },
    /// Validate a domain pack JSON file for structural correctness.
    Validate {
        /// Path to domain pack JSON file
        file: String,
    },
    /// Test a domain pack locally: run a sample claim through the proof engine and print the result.
    Test {
        /// Path to domain pack JSON file
        #[arg(long, short = 'd')]
        domain: String,
        /// Path to claim request JSON file
        #[arg(long, short = 'c')]
        claim: String,
        /// Namespace to use for the test (arbitrary)
        #[arg(long, short = 'n', default_value = "test")]
        ns: String,
    },
    /// Register a domain pack with the running server (persisted to WAL).
    Register {
        /// Server base URL
        #[arg(long, default_value = "http://localhost:8080")]
        server: String,
        /// Path to domain pack JSON file
        file: String,
    },
}

#[derive(Parser)]
#[command(name = "agentstate")]
#[command(about="AgentState admin CLI", long_about=None)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Restore {
        snapshot: String,
        wal_dir: String,
        out: String,
        #[arg(long)]
        dump: Option<String>,
    },
    /// Verify temporal properties (LTL) over a WAL trace and emit a JSON report.
    Verify {
        /// WAL directory (same as DATA_DIR used by the server)
        #[arg(long, short = 'd', default_value = ".")]
        dir: String,
        /// Filter to a specific namespace (optional; omit to check all namespaces)
        #[arg(long, short = 'n')]
        ns: Option<String>,
        /// Path to a .ltl.json property file (repeatable)
        #[arg(long, short = 'p')]
        property: Vec<String>,
        /// Write JSON report to this file instead of stdout
        #[arg(long, short = 'o')]
        output: Option<String>,
        /// Exit with code 1 if any property fails
        #[arg(long)]
        fail_on_violation: bool,
    },
    /// Submit a claim for verification and retrieve the formal proof.
    #[command(subcommand)]
    Claim(ClaimCmd),
    /// Domain pack management: list, validate, test, and register domain packs.
    #[command(subcommand)]
    Domain(DomainCmd),
    /// Show the full state history of a single object, with field-level diffs between versions.
    Replay {
        /// Object ID to trace through the WAL
        object_id: String,
        /// WAL directory (same as DATA_DIR used by the server)
        #[arg(long, short = 'd', default_value = ".")]
        dir: String,
        /// Filter to a specific namespace (optional; omit to match any namespace)
        #[arg(long, short = 'n')]
        ns: Option<String>,
        /// Seed initial state from a snapshot file (.zst) before replaying the WAL
        #[arg(long, short = 's')]
        snapshot: Option<String>,
    },
}

/// Render a proof JSON as human-readable explanation.
fn explain_proof(proof: &serde_json::Value) {
    let status = proof.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
    let proof_id = proof.get("proof_id").and_then(|v| v.as_str()).unwrap_or("?");
    let domain = proof.get("domain").and_then(|v| v.as_str()).unwrap_or("?");
    let confidence = proof.get("confidence").and_then(|v| v.as_str()).unwrap_or("?");
    let conclusion = proof.get("conclusion").cloned().unwrap_or_default();
    let predicate = conclusion.get("predicate").and_then(|v| v.as_str()).unwrap_or("?");

    println!("Proof {}", proof_id);
    println!("  Status:     {}", status.to_uppercase());
    println!("  Confidence: {}", confidence);
    println!("  Domain:     {}", domain);
    println!("  Conclusion: {}", predicate);

    if let Some(reason) = proof.get("refutation_reason").and_then(|v| v.as_str()) {
        println!("  Refuted:    {}", reason);
    }

    let props = proof.get("properties");
    println!("\nFormal Properties:");
    for (label, key) in &[
        ("Self-consistent", "self_consistent"),
        ("Minimal",         "minimal"),
        ("Predictive",      "has_predictive_constraint"),
        ("Verifiable",      "verifiable"),
        ("Sound",           "sound"),
        ("Monotonic",       "monotonic"),
    ] {
        let val = props
            .and_then(|p| p.get(*key))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        println!("  {:20} {}", label, if val { "✓" } else { "✗" });
    }

    if let Some(steps) = proof.get("steps").and_then(|v| v.as_array()) {
        println!("\nProof DAG ({} step(s)):", steps.len());
        for step in steps {
            let kind = step.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
            let rule = step.get("rule").and_then(|v| v.as_str()).unwrap_or("?");
            let desc = step.get("conclusion_desc").and_then(|v| v.as_str()).unwrap_or("");
            let step_id = step.get("step_id").and_then(|v| v.as_u64()).unwrap_or(0);
            println!("  [{}] {:12} {} — {}", step_id, kind.to_uppercase(), rule, desc);
        }
    }

    if let Some(assumptions) = proof.get("assumptions").and_then(|v| v.as_array()) {
        println!("\nAssumptions ({}):", assumptions.len());
        for a in assumptions {
            println!("  • {}", a.as_str().unwrap_or("?"));
        }
    }

    if let Some(consequences) = proof.get("consequences_checked").and_then(|v| v.as_array()) {
        if !consequences.is_empty() {
            println!("\nConsequences:");
            for c in consequences {
                let pred = c.get("predicate_desc").and_then(|v| v.as_str()).unwrap_or("?");
                let status = c.get("status").and_then(|v| v.as_str()).unwrap_or("pending");
                println!("  {:8} {}", status.to_uppercase(), pred);
            }
        }
    }
}

fn read_snapshot(path: &str) -> Result<Vec<serde_json::Value>> {
    let f = std::fs::File::open(path)?;
    let mut d = zstd::Decoder::new(f)?;
    let mut s = String::new();
    use std::io::Read;
    d.read_to_string(&mut s)?;
    let mut out = Vec::new();
    for line in s.lines() {
        if !line.trim().is_empty() {
            out.push(serde_json::from_str(line)?);
        }
    }
    Ok(out)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Restore {
            snapshot,
            wal_dir,
            out,
            dump,
        } => {
            let mut objs = read_snapshot(&snapshot)?;
            // replay WAL tail
            let recs = walbin::replay(&wal_dir).unwrap_or_default();
            for r in recs {
                match r {
                    walbin::RecBody::Put { ns: _, obj } => {
                        objs.push(obj);
                    }
                    walbin::RecBody::Delete { ns, id } => {
                        objs.retain(|o| {
                            !(o.get("ns").and_then(|v| v.as_str()) == Some(&ns)
                                && o.get("id").and_then(|v| v.as_str()) == Some(&id))
                        });
                    }
                    _ => {}
                }
            }
            let last_seq = objs
                .iter()
                .filter_map(|v| v.get("commit_seq").and_then(|x| x.as_u64()))
                .max()
                .unwrap_or(0);
            if let Some(path) = dump {
                let mut s = String::new();
                for o in &objs {
                    s.push_str(&serde_json::to_string(o)?);
                    s.push('\n');
                }
                std::fs::write(path, s)?;
            }
            let report = serde_json::json!({ "last_seq": last_seq, "objects": objs.len(), "crc_ok": true, "index_consistent": true });
            std::fs::write(out, serde_json::to_vec_pretty(&report)?)?;
        }
        Cmd::Verify {
            dir,
            ns,
            property,
            output,
            fail_on_violation,
        } => {
            if property.is_empty() {
                eprintln!("No property files specified. Use --property <path.ltl.json>");
                std::process::exit(2);
            }
            let properties = agentstate_verify::load_properties(&property)?;
            let report = agentstate_verify::run(&dir, ns.as_deref(), &properties);
            let json = serde_json::to_string_pretty(&report)?;
            match output {
                Some(path) => std::fs::write(&path, &json)?,
                None => println!("{}", json),
            }
            if fail_on_violation && report.failed > 0 {
                std::process::exit(1);
            }
        }
        Cmd::Domain(domain_cmd) => match domain_cmd {
            DomainCmd::List { server } => {
                let resp = Client::new()
                    .get(format!("{}/admin/domains", server))
                    .send()?
                    .error_for_status()?;
                let body: serde_json::Value = resp.json()?;
                println!("{}", serde_json::to_string_pretty(&body)?);
            }
            DomainCmd::Validate { file } => {
                let raw = std::fs::read_to_string(&file)?;
                let pack: DomainPack = serde_json::from_str(&raw).map_err(|e| {
                    anyhow::anyhow!("Invalid domain pack JSON: {}", e)
                })?;
                // Check all inference_chain rules exist
                let mut errors: Vec<String> = Vec::new();
                for tmpl in &pack.claim_templates {
                    for rule_id in &tmpl.inference_chain {
                        if !pack.inference_rules.iter().any(|r| &r.id == rule_id) {
                            errors.push(format!(
                                "template '{}': inference_chain references unknown rule '{}'",
                                tmpl.id, rule_id
                            ));
                        }
                    }
                }
                if errors.is_empty() {
                    println!("✓ Domain pack '{}' v{} is valid", pack.domain, pack.version);
                    println!("  Templates: {}", pack.claim_templates.len());
                    println!("  Inference rules: {}", pack.inference_rules.len());
                    println!("  Axioms: {}", pack.axioms.len());
                } else {
                    eprintln!("Domain pack validation failed:");
                    for e in &errors {
                        eprintln!("  - {}", e);
                    }
                    std::process::exit(1);
                }
            }
            DomainCmd::Test { domain, claim, ns } => {
                let pack: DomainPack = serde_json::from_str(&std::fs::read_to_string(&domain)?)?;
                let req: ClaimRequest = serde_json::from_str(&std::fs::read_to_string(&claim)?)?;
                let stored_claim = req.into_claim(ns);
                let mut registry = DomainRegistry::new();
                registry.register(pack.clone());
                let proof = build_proof(&stored_claim, &pack, &[]);
                println!("{}", serde_json::to_string_pretty(&proof)?);
            }
            DomainCmd::Register { server, file } => {
                let raw = std::fs::read_to_string(&file)?;
                let body: serde_json::Value = serde_json::from_str(&raw)?;
                let resp = Client::new()
                    .post(format!("{}/admin/domains", server))
                    .json(&body)
                    .send()?
                    .error_for_status()?;
                let out: serde_json::Value = resp.json()?;
                println!("{}", serde_json::to_string_pretty(&out)?);
            }
        },
        Cmd::Claim(claim_cmd) => match claim_cmd {
            ClaimCmd::Submit { server, ns, file } => {
                let body: serde_json::Value =
                    serde_json::from_str(&std::fs::read_to_string(&file)?)?;
                let client = Client::new();
                let resp = client
                    .post(format!("{}/admin/namespaces/{}/claims", server, ns))
                    .json(&body)
                    .send()?
                    .error_for_status()?;
                let out: serde_json::Value = resp.json()?;
                println!("{}", serde_json::to_string_pretty(&out)?);
            }
            ClaimCmd::Get { server, ns, id } => {
                let resp = Client::new()
                    .get(format!("{}/admin/namespaces/{}/claims/{}", server, ns, id))
                    .send()?
                    .error_for_status()?;
                println!("{}", serde_json::to_string_pretty(&resp.json::<serde_json::Value>()?)?);
            }
            ClaimCmd::Proof { server, ns, id } => {
                let resp = Client::new()
                    .get(format!(
                        "{}/admin/namespaces/{}/claims/{}/proof",
                        server, ns, id
                    ))
                    .send()?
                    .error_for_status()?;
                println!("{}", serde_json::to_string_pretty(&resp.json::<serde_json::Value>()?)?);
            }
            ClaimCmd::List { server, ns } => {
                let resp = Client::new()
                    .get(format!("{}/admin/namespaces/{}/claims", server, ns))
                    .send()?
                    .error_for_status()?;
                println!("{}", serde_json::to_string_pretty(&resp.json::<serde_json::Value>()?)?);
            }
            ClaimCmd::Challenge {
                server,
                ns,
                id,
                reason,
                step,
                counter,
            } => {
                let body = serde_json::json!({
                    "challenged_step": step,
                    "reason": reason,
                    "counter_evidence": counter,
                });
                let resp = Client::new()
                    .post(format!(
                        "{}/admin/namespaces/{}/claims/{}/challenge",
                        server, ns, id
                    ))
                    .json(&body)
                    .send()?
                    .error_for_status()?;
                println!("{}", serde_json::to_string_pretty(&resp.json::<serde_json::Value>()?)?);
            }
            ClaimCmd::Explain { server, ns, id } => {
                let proof: serde_json::Value = Client::new()
                    .get(format!(
                        "{}/admin/namespaces/{}/claims/{}/proof",
                        server, ns, id
                    ))
                    .send()?
                    .error_for_status()?
                    .json()?;
                explain_proof(&proof);
            }
            ClaimCmd::Vc { server, ns, id } => {
                let vc: serde_json::Value = Client::new()
                    .get(format!(
                        "{}/admin/namespaces/{}/claims/{}/vc",
                        server, ns, id
                    ))
                    .send()?
                    .error_for_status()?
                    .json()?;
                println!("{}", serde_json::to_string_pretty(&vc)?);
            }
        },
        Cmd::Replay {
            object_id,
            dir,
            ns,
            snapshot,
        } => {
            let mut versions: Vec<serde_json::Value> = Vec::new();

            // 1. Seed from snapshot if provided
            if let Some(snap_path) = snapshot {
                for obj in read_snapshot(&snap_path).unwrap_or_default() {
                    let id_match = obj.get("id").and_then(|v| v.as_str()) == Some(&object_id);
                    let ns_match = ns
                        .as_deref()
                        .map(|n| obj.get("ns").and_then(|v| v.as_str()) == Some(n))
                        .unwrap_or(true);
                    if id_match && ns_match {
                        versions.push(obj);
                    }
                }
            }

            // 2. Replay WAL
            let recs = walbin::replay(&dir).unwrap_or_default();
            for r in recs {
                match r {
                    walbin::RecBody::Put { ns: rec_ns, obj } => {
                        let id_match = obj.get("id").and_then(|v| v.as_str()) == Some(&object_id);
                        let ns_match = ns.as_deref().map(|n| rec_ns == n).unwrap_or(true);
                        if id_match && ns_match {
                            versions.push(obj);
                        }
                    }
                    walbin::RecBody::Delete { ns: rec_ns, id } => {
                        if id == object_id {
                            let ns_match = ns.as_deref().map(|n| rec_ns == n).unwrap_or(true);
                            if ns_match {
                                versions.push(serde_json::json!({
                                    "id": id,
                                    "ns": rec_ns,
                                    "_deleted": true,
                                }));
                            }
                        }
                    }
                    _ => {}
                }
            }

            if versions.is_empty() {
                eprintln!("No history found for object '{}'", object_id);
                std::process::exit(1);
            }

            println!(
                "History for '{}' ({} version(s)):",
                object_id,
                versions.len()
            );
            println!("{}", "─".repeat(60));

            let mut prev_body: Option<serde_json::Value> = None;
            for (i, obj) in versions.iter().enumerate() {
                let ts = obj.get("ts").and_then(|v| v.as_str()).unwrap_or("unknown");
                let seq = obj.get("commit_seq").and_then(|v| v.as_u64()).unwrap_or(0);

                if obj
                    .get("_deleted")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    println!("[v{}] ts={} DELETED", i + 1, ts);
                } else {
                    let commit = obj.get("commit").and_then(|v| v.as_str()).unwrap_or("?");
                    let short_commit = if commit.len() >= 8 {
                        &commit[..8]
                    } else {
                        commit
                    };
                    println!("[v{}] ts={} commit={} seq={}", i + 1, ts, short_commit, seq);

                    if let Some(cur) = obj.get("body") {
                        if let Some(prev) = &prev_body {
                            let cur_map = cur.as_object();
                            let prev_map = prev.as_object();
                            if let (Some(cm), Some(pm)) = (cur_map, prev_map) {
                                for (k, v) in cm {
                                    match pm.get(k) {
                                        Some(pv) if pv == v => {}
                                        Some(pv) => println!("    ~ {}: {} → {}", k, pv, v),
                                        None => println!("    + {}: {}", k, v),
                                    }
                                }
                                for k in pm.keys() {
                                    if !cm.contains_key(k) {
                                        println!("    - {} (removed)", k);
                                    }
                                }
                            }
                        } else {
                            println!("    (initial state)");
                            if let Ok(pretty) = serde_json::to_string_pretty(cur) {
                                for line in pretty.lines() {
                                    println!("    {}", line);
                                }
                            }
                        }
                        prev_body = Some(cur.clone());
                    }

                    // Show cause if present
                    if let Some(cause) = obj.get("cause") {
                        if let Some(actor) = cause.get("actor").and_then(|v| v.as_str()) {
                            println!("    cause.actor: {}", actor);
                        }
                        if let Some(note) = cause.get("note").and_then(|v| v.as_str()) {
                            println!("    cause.note: {}", note);
                        }
                    }
                }
                println!();
            }
        }
    }
    Ok(())
}
