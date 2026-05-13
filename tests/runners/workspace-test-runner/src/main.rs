//! Workspace Test Runner
//!
//! Maps test-case IDs (TC-XX-YYY) from markdown specifications to actual
//! `cargo test` results and produces a JSON coverage report.
//!
//! Usage:
//!   w-test-runner [--workspace <path>] [--json <file>]

use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize)]
struct TestReport {
    meta: ReportMeta,
    summary: Summary,
    specs: Vec<SpecEntry>,
    unmapped_tests: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ReportMeta {
    generated_at: String,
    workspace: String,
    runner_version: String,
}

#[derive(Debug, Serialize)]
struct Summary {
    total_specs: usize,
    implemented: usize,
    not_implemented: usize,
    passed: usize,
    failed: usize,
    coverage_percent: f64,
}

#[derive(Debug, Serialize)]
struct SpecEntry {
    tc_id: String,
    phase: String,
    title: String,
    test_name: Option<String>,
    status: SpecStatus,
}

#[derive(Debug, Serialize, PartialEq)]
enum SpecStatus {
    Passed,
    Failed,
    NotImplemented,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let workspace = arg_value(&args, "--workspace").unwrap_or_else(|| "src".into());
    let json_out = arg_value(&args, "--json");
    let specs_dir = arg_value(&args, "--specs").unwrap_or_else(|| "tests".into());

    println!("Workspace Test Runner v0.1.0");
    println!("Workspace: {workspace}");
    println!("Specs dir: {specs_dir}");

    // 1. Parse specification files.
    let specs = parse_specs(&specs_dir);
    println!("Found {} test-case specifications", specs.len());

    // 2. Run cargo test and capture results.
    let test_results = run_cargo_test(&workspace);
    println!("Cargo test executed: {} results captured", test_results.len());

    // 3. Map specs to tests.
    let mut entries = Vec::with_capacity(specs.len());
    let mut implemented = 0usize;
    let mut passed = 0usize;
    let mut failed = 0usize;

    for (tc_id, (phase, title)) in specs {
        let normalized = tc_id.to_lowercase().replace("-", "_");
        let matching = test_results
            .iter()
            .find(|(name, _)| name.contains(&normalized));

        let (status, test_name) = match matching {
            Some((name, true)) => {
                implemented += 1;
                passed += 1;
                (SpecStatus::Passed, Some(name.clone()))
            }
            Some((name, false)) => {
                implemented += 1;
                failed += 1;
                (SpecStatus::Failed, Some(name.clone()))
            }
            None => (SpecStatus::NotImplemented, None),
        };

        entries.push(SpecEntry {
            tc_id: tc_id.clone(),
            phase,
            title,
            test_name,
            status,
        });
    }

    let not_implemented = entries.len() - implemented;
    let coverage = if !entries.is_empty() {
        (implemented as f64 / entries.len() as f64) * 100.0
    } else {
        0.0
    };

    // Collect unmapped tests (tests without a TC-ID in specs).
    let mapped_names: std::collections::HashSet<_> = entries
        .iter()
        .filter_map(|e| e.test_name.clone())
        .collect();
    let unmapped_tests: Vec<String> = test_results
        .into_keys()
        .filter(|n| !mapped_names.contains(n))
        .collect();

    let report = TestReport {
        meta: ReportMeta {
            generated_at: chrono_now_string(),
            workspace: workspace.clone(),
            runner_version: "0.1.0".into(),
        },
        summary: Summary {
            total_specs: entries.len(),
            implemented,
            not_implemented,
            passed,
            failed,
            coverage_percent: coverage,
        },
        specs: entries,
        unmapped_tests,
    };

    let json = serde_json::to_string_pretty(&report).unwrap();

    if let Some(path) = json_out {
        fs::write(&path, &json).expect("Failed to write JSON report");
        println!("Report written to: {path}");
    } else {
        println!("\n{json}");
    }

    println!(
        "\nSummary: {} specs, {} implemented ({:.1}%), {} passed, {} failed",
        report.summary.total_specs,
        report.summary.implemented,
        report.summary.coverage_percent,
        report.summary.passed,
        report.summary.failed,
    );

    if report.summary.failed > 0 {
        std::process::exit(1);
    }
}

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1).cloned())
}

fn chrono_now_string() -> String {
    // Simple ISO-like timestamp without external chrono dependency.
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let ss = rem % 60;
    format!("{}T{:02}:{:02}:{:02}Z", days, hh, mm, ss)
}

/// Parse all `tests/phase-*.md` files and extract TC-IDs.
fn parse_specs(dir: &str) -> HashMap<String, (String, String)> {
    let mut map = HashMap::new();
    let tc_re = Regex::new(r"TC-\d{2}-\d{3}").unwrap();
    let title_re = Regex::new(r"###\s+(TC-\d{2}-\d{3}):\s*(.+)").unwrap();

    let root = Path::new(dir);
    if !root.exists() {
        eprintln!("Warning: specs directory '{}' does not exist", dir);
        return map;
    }

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
    {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        let file_name = entry
            .path()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        for cap in tc_re.find_iter(&content) {
            let tc_id = cap.as_str().to_string();
            // Try to extract title from the same line or nearby.
            let title = title_re
                .captures(&content)
                .and_then(|c| c.get(2))
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_else(|| "(no title)".into());

            map.insert(tc_id, (file_name.clone(), title.clone()));
        }
    }

    map
}

/// Run `cargo test` and parse output lines like:
/// `test result: ok. 42 passed; 0 failed;`
/// plus individual `test <name> ... ok/failed`.
fn run_cargo_test(workspace: &str) -> HashMap<String, bool> {
    let mut results = HashMap::new();

    let output = Command::new("cargo")
        .args(&["test", "--all-targets", "--", "--nocapture"])
        .current_dir(workspace)
        .env("RUST_BACKTRACE", "1")
        .output();

    let (stdout, stderr) = match output {
        Ok(o) => (
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        ),
        Err(e) => {
            eprintln!("Failed to run cargo test: {e}");
            return results;
        }
    };

    let combined = format!("{}\n{}", stdout, stderr);
    let test_re = Regex::new(r"test\s+([\w:]+)\s+\.\.\.\s+(ok|FAILED)").unwrap();

    for cap in test_re.captures_iter(&combined) {
        let name = cap[1].to_string();
        let ok = &cap[2] == "ok";
        results.insert(name, ok);
    }

    results
}
