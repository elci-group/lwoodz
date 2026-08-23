// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

use clap::Parser;
use lwoodz::cli::{LwoodzArgs as Cli, LwoodzCommand};
use lwoodz::config::loader::OperationMode;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(e) = lwoodz::util::dotenv::load() {
        tracing::warn!(?e, "dotenv load failed");
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // Handle init subcommand without requiring existing config
    if matches!(cli.command, Some(LwoodzCommand::Init)) {
        return init_config();
    }

    let mut config = if let Some(p) = cli.config.as_ref() {
        lwoodz::config::load_from(Some(p))?
    } else {
        lwoodz::config::load()?
    };

    if cli.force {
        config.daemon.startup_guard = false;
    }

    // Handle subcommands
    match cli.command {
        Some(LwoodzCommand::Daemon) => {
            let daemon = lwoodz::daemon::Daemon::new(config);
            daemon.run().await?;
        }
        Some(LwoodzCommand::Remedy { dry_run }) => {
            cmd_generate(&config, dry_run, cli.json).await?;
        }
        Some(LwoodzCommand::Check) => {
            let manifest = lwoodz::manifest::scan(&config.repo_path);
            let pairs: Vec<(String, String)> = manifest
                .dependencies
                .iter()
                .filter_map(|d| d.license.as_ref().map(|l| (d.name.clone(), l.clone())))
                .collect();
            let rep = lwoodz::license::compatibility::build_report(
                &lwoodz::license::spdx::normalize_spdx(&config.project.license),
                &pairs,
            );
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&rep)?);
            } else {
                println!(
                    "Project license: {} | deps: {} | incompatible: {} | warnings: {}",
                    rep.project_license, rep.total_deps, rep.incompatible_count, rep.warning_count
                );
                for i in &rep.issues {
                    let icon =
                        if i.compatibility == lwoodz::license::compatibility::Compatibility::Incompatible {
                            "x"
                        } else {
                            "!"
                        };
                    println!(
                        "  {} {} ({}) - {}",
                        icon, i.dependency, i.dep_license, i.reason
                    );
                }
                if rep.is_clean() {
                    println!("  No issues.");
                }
            }
            if rep.has_blockers() {
                std::process::exit(2);
            }
        }
        Some(LwoodzCommand::Audit) | None => {
            // Default to audit if no subcommand provided
            let report = lwoodz::audit::run(&config)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_audit(&report);
            }
            if !report.passed && config.audit.fail_on_incompatible {
                std::process::exit(1);
            }
        }
        Some(LwoodzCommand::Init) => {
            // Already handled above
            unreachable!();
        }
    }

    Ok(())
}

async fn cmd_generate(
    config: &lwoodz::config::Config,
    dry_run: bool,
    json: bool,
) -> anyhow::Result<()> {
    if dry_run {
        let body = lwoodz::generate::preview_license_body(config)?;
        let spdx = lwoodz::license::spdx::normalize_spdx(&config.project.license);
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "spdx": spdx,
                    "holder": config.project.copyright_holder,
                    "year": config.project.copyright_year,
                    "preview": body.lines().take(20).collect::<Vec<_>>()
                })
            );
        } else {
            println!("Dry run -- would generate LICENSE ({})", spdx);
            println!("--- LICENSE preview (first 20 lines) ---");
            for line in body.lines().take(20) {
                println!("{}", line);
            }
            println!("--- would write to: {} ---", config.generate.license_file);
        }
        return Ok(());
    }

    let res = lwoodz::generate::generate_license_file(config)?;
    if json {
        let mut summary = serde_json::json!({
            "license": res.license_path.display().to_string(),
            "spdx": res.spdx,
            "notice": res.notice_path.map(|p| p.display().to_string()),
            "copyright": res.copyright_path.map(|p| p.display().to_string()),
            "attribution": res.attribution_path.map(|p| p.display().to_string()),
            "manifest": res.manifest_path.map(|p| p.display().to_string()),
            "written": res.written.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "skipped": res.skipped.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        });

        if config.headers.enabled && config.operation.mode == OperationMode::Enforce {
            let hc = lwoodz::license::header::HeaderConfig {
                holder: config.project.copyright_holder.clone(),
                year: config.project.copyright_year,
                spdx: lwoodz::license::spdx::normalize_spdx(&config.project.license),
                insert_spdx: config.headers.insert_spdx,
            };
            let results = lwoodz::license::header::ensure_headers(
                &config.repo_path,
                &hc,
                &config.headers.exclude,
                &config.headers.include,
            )?;
            let inserted = results
                .iter()
                .filter(|r| r.action == lwoodz::license::header::HeaderAction::Inserted)
                .count();
            let updated = results
                .iter()
                .filter(|r| r.action == lwoodz::license::header::HeaderAction::Updated)
                .count();
            let scanned = results.len();
            summary["headers"] = serde_json::json!({
                "inserted": inserted,
                "updated": updated,
                "scanned": scanned,
            });
        }

        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        let action_label = if res.written.contains(&res.license_path) {
            "Generated"
        } else {
            "Skipped"
        };
        println!(
            "{} {} ({})",
            action_label,
            res.license_path.display(),
            res.spdx
        );
        if let Some(p) = res.notice_path {
            println!(
                "  NOTICE -> {} ({})",
                p.display(),
                if res.written.contains(&p) {
                    "written"
                } else {
                    "skipped"
                }
            );
        }
        if let Some(p) = res.copyright_path {
            println!(
                "  COPYRIGHT -> {} ({})",
                p.display(),
                if res.written.contains(&p) {
                    "written"
                } else {
                    "skipped"
                }
            );
        }
        if let Some(p) = res.attribution_path {
            println!(
                "  Attribution -> {} ({})",
                p.display(),
                if res.written.contains(&p) {
                    "written"
                } else {
                    "skipped"
                }
            );
        }
        if let Some(p) = res.manifest_path {
            println!(
                "  SPDX manifest -> {} ({})",
                p.display(),
                if res.written.contains(&p) {
                    "written"
                } else {
                    "skipped"
                }
            );
        }

        if config.headers.enabled && config.operation.mode == OperationMode::Enforce {
            let hc = lwoodz::license::header::HeaderConfig {
                holder: config.project.copyright_holder.clone(),
                year: config.project.copyright_year,
                spdx: lwoodz::license::spdx::normalize_spdx(&config.project.license),
                insert_spdx: config.headers.insert_spdx,
            };
            let results = lwoodz::license::header::ensure_headers(
                &config.repo_path,
                &hc,
                &config.headers.exclude,
                &config.headers.include,
            )?;
            let inserted = results
                .iter()
                .filter(|r| r.action == lwoodz::license::header::HeaderAction::Inserted)
                .count();
            let updated = results
                .iter()
                .filter(|r| r.action == lwoodz::license::header::HeaderAction::Updated)
                .count();
            println!(
                "  Headers: {} inserted, {} updated ({} files scanned)",
                inserted,
                updated,
                results.len()
            );
        }
    }
    Ok(())
}

fn print_audit(report: &lwoodz::audit::AuditReport) {
    println!(
        "lwoodz audit — {} ({})",
        report.project_license,
        if report.passed { "PASS" } else { "FAIL" }
    );
    println!("  Holder: {} ({})", report.holder, report.year);
    println!(
        "  LICENSE file: {} (detected: {})",
        if report.has_license_file {
            "present"
        } else {
            "MISSING"
        },
        report.detected_license.as_deref().unwrap_or("unknown")
    );
    println!(
        "  SPDX valid: {}  manifest: {}",
        report.spdx_valid,
        if report.spdx_manifest_exists {
            "present"
        } else {
            "missing"
        }
    );
    println!(
        "  Dependencies: {} total ({} third-party, {} first-party), {} incompatible, {} warnings",
        report.compatibility.total_deps,
        report.compatibility.third_party,
        report.compatibility.first_party,
        report.compatibility.incompatible,
        report.compatibility.warnings
    );
    if let Some(hc) = &report.header_coverage {
        println!(
            "  Headers: {}/{} files have copyright/SPDX",
            hc.with_header, hc.total_files
        );
    }
    if !report.findings.is_empty() {
        println!("\nFindings ({}):", report.findings.len());
        for f in &report.findings {
            let icon = match f.level.as_str() {
                "error" => "✗",
                "warning" => "⚠",
                _ => "·",
            };
            println!("  {} [{}] {} — {}", icon, f.level, f.code, f.message);
        }
    } else {
        println!("\n  No findings — repository is compliant.");
    }
}

fn init_config() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let dest = cwd.join("lwoodz.toml");
    if dest.exists() {
        anyhow::bail!("lwoodz.toml already exists at {}", dest.display());
    }
    let holder = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "Your Name".to_string());
    let year = lwoodz::util::current_year();
    let project_name = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("myproject");

    let content = format!(
        r#"# lwoodz configuration — licensing daemon for {project_name}
# Lwoodz is to software licensing what Kaptaind is to version management.
# Copy this file to lwoodz.toml in your repo root and adjust values.

[project]
# SPDX identifier for the project. See https://spdx.org/licenses/
license = "MIT"
copyright_holder = "{holder}"
copyright_year = {year}
project_name = "{project_name}"
# copyright_holder_email = "you@example.com"
# project_url = "https://github.com/elci-group/{project_name}"
# dual_license = "Apache-2.0"          # second license for dual-licensing
# commercial_license = "SEE LICENSE IN Commercial-LICENSE"

[operation]
# observe = report only, enforce = generate/update files
mode = "observe"

[watch]
path = "."
recursive = true
ignore_file = ".lwoodignore"

[generate]
license_file = "LICENSE"
notice_file = "NOTICE"
copyright_file = "COPYRIGHT"
attribution_file = "THIRD_PARTY_NOTICES"
# When false, existing files are left untouched. When true, files are overwritten.
overwrite = true

[headers]
enabled = true
insert_spdx = true
exclude = ["target/**", "node_modules/**", "dist/**", ".git/**", ".lwoodz/**", "vendor/**"]
# include = ["src/**"]

[compatibility]
enforce = true
policy = "permissive"  # permissive | copyleft-allowed | strict

[daemon]
startup_guard = false
shutdown_grace_secs = 10

[inference]
enabled = true
provider = "groq"
model = "auto"
groq_model = "llama-3.3-70b-versatile"
# api_key_env = "GROQ_API_KEY"  # env var holding the Groq API key

[spdx]
produce_manifest = true
manifest_path = ".lwoodz/manifest.json"
include_transitive = false

[audit]
enabled = true
log_path = ".lwoodz/audit.jsonl"
fail_on_incompatible = false
"#
    );
    std::fs::write(&dest, content)?;
    println!("✓ Created lwoodz.toml at {}", dest.display());
    println!("  Edit [project] license/holder, then run `lwoodz remedy` or `lwoodz audit`.");
    Ok(())
}
