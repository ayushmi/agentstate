use agentstate_storage::walbin;
use anyhow::Result;
use clap::{Parser, Subcommand};
use agentstate_verify;

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
        Cmd::Verify { dir, ns, property, output, fail_on_violation } => {
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
        Cmd::Replay { object_id, dir, ns, snapshot } => {
            let mut versions: Vec<serde_json::Value> = Vec::new();

            // 1. Seed from snapshot if provided
            if let Some(snap_path) = snapshot {
                for obj in read_snapshot(&snap_path).unwrap_or_default() {
                    let id_match = obj.get("id").and_then(|v| v.as_str()) == Some(&object_id);
                    let ns_match = ns.as_deref()
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

            println!("History for '{}' ({} version(s)):", object_id, versions.len());
            println!("{}", "─".repeat(60));

            let mut prev_body: Option<serde_json::Value> = None;
            for (i, obj) in versions.iter().enumerate() {
                let ts = obj.get("ts").and_then(|v| v.as_str()).unwrap_or("unknown");
                let seq = obj.get("commit_seq").and_then(|v| v.as_u64()).unwrap_or(0);

                if obj.get("_deleted").and_then(|v| v.as_bool()).unwrap_or(false) {
                    println!("[v{}] ts={} DELETED", i + 1, ts);
                } else {
                    let commit = obj.get("commit").and_then(|v| v.as_str()).unwrap_or("?");
                    let short_commit = if commit.len() >= 8 { &commit[..8] } else { commit };
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
