// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

use clap::{CommandFactory, Parser};
use lwoodz::cli::{LwoodzCliArgs as Cli, LwoodzCliCommand as Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = lwoodz::util::dotenv::load();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    let config = if let Some(p) = cli.config.as_ref() {
        lwoodz::config::load_from(Some(p))?
    } else {
        lwoodz::config::load()?
    };
    match cli.command {
        Commands::Init { force } => cmd_init(force),
        Commands::Remedy { dry_run } => cmd_generate(&config, dry_run, cli.json),
        Commands::Audit => cmd_audit(&config, cli.json),
        Commands::Check => cmd_check(&config, cli.json),
        Commands::Licenses => cmd_licenses(cli.json),
        Commands::Compat { project, dep } => cmd_compat(project, dep, cli.json),
        Commands::Detect { path } => cmd_detect(path, cli.json),
        Commands::Explain { spdx } => cmd_explain(&config, spdx).await,
        Commands::Headers { dry_run } => cmd_headers(&config, dry_run),
        Commands::Dual { license } => cmd_dual(&config, license),
        Commands::Status => cmd_status(&config, cli.json),
        Commands::Version => {
            println!("lwoodz-cli {}", lwoodz::VERSION);
            Ok(())
        }
        Commands::Completions { shell } => cmd_completions(shell),
    }
}

fn cmd_completions(shell: clap_complete::Shell) -> anyhow::Result<()> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
    Ok(())
}

fn cmd_init(force: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let dest = cwd.join("lwoodz.toml");
    if dest.exists() && !force {
        anyhow::bail!(
            "lwoodz.toml already exists (use --force) at {}",
            dest.display()
        );
    }
    if dest.exists() && force {
        std::fs::remove_file(&dest)?;
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
        r#"# lwoodz configuration -- licensing daemon for {project_name}

[project]
license = "MIT"
copyright_holder = "{holder}"
copyright_year = {year}
project_name = "{project_name}"
# copyright_holder_email = "you@example.com"
# project_url = "https://github.com/elci-group/{project_name}"
# dual_license = "Apache-2.0"
# commercial_license = "SEE LICENSE IN Commercial-LICENSE"

[operation]
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

[compatibility]
enforce = true
policy = "permissive"

[daemon]
startup_guard = false
shutdown_grace_secs = 10

[inference]
enabled = true
provider = "groq"
model = "auto"
groq_model = "llama-3.3-70b-versatile"

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
    println!("Created lwoodz.toml at {}", dest.display());
    Ok(())
}

fn cmd_generate(cfg: &lwoodz::config::Config, dry_run: bool, json: bool) -> anyhow::Result<()> {
    if dry_run {
        let body = lwoodz::generate::preview_license_body(cfg)?;
        let spdx = lwoodz::license::spdx::normalize_spdx(&cfg.project.license);
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "spdx": spdx,
                    "holder": cfg.project.copyright_holder,
                    "year": cfg.project.copyright_year,
                    "preview": body.lines().take(20).collect::<Vec<_>>()
                })
            );
        } else {
            println!("Dry run -- would generate LICENSE ({})", spdx);
            println!("--- LICENSE preview (first 20 lines) ---");
            for line in body.lines().take(20) {
                println!("{}", line);
            }
            println!("--- would write to: {} ---", cfg.generate.license_file);
        }
        return Ok(());
    }
    let res = lwoodz::generate::generate_license_file(cfg)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "license": res.license_path.display().to_string(),
                "spdx": res.spdx,
                "notice": res.notice_path.map(|p| p.display().to_string()),
                "copyright": res.copyright_path.map(|p| p.display().to_string()),
                "attribution": res.attribution_path.map(|p| p.display().to_string()),
                "manifest": res.manifest_path.map(|p| p.display().to_string()),
                "written": res.written.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                "skipped": res.skipped.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            }))?
        );
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
    }
    Ok(())
}

fn cmd_audit(cfg: &lwoodz::config::Config, json: bool) -> anyhow::Result<()> {
    let report = lwoodz::audit::run(cfg)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_audit(&report);
    }
    if !report.passed && cfg.audit.fail_on_incompatible {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_check(cfg: &lwoodz::config::Config, json: bool) -> anyhow::Result<()> {
    let manifest = lwoodz::manifest::scan(&cfg.repo_path);
    let pairs: Vec<(String, String)> = manifest
        .dependencies
        .iter()
        .filter_map(|d| d.license.as_ref().map(|l| (d.name.clone(), l.clone())))
        .collect();
    let rep = lwoodz::license::compatibility::build_report(
        &lwoodz::license::spdx::normalize_spdx(&cfg.project.license),
        &pairs,
    );
    if json {
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
    Ok(())
}

fn cmd_licenses(json: bool) -> anyhow::Result<()> {
    let list = lwoodz::license::templates::all_licenses();
    if json {
        let j: Vec<_> = list
            .iter()
            .map(|t| serde_json::json!({ "spdx": t.spdx, "name": t.name }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&j)?);
    } else {
        println!("Known licenses ({}):", list.len());
        for t in &list {
            println!("  {:20}  {}", t.spdx, t.name);
        }
    }
    Ok(())
}

fn cmd_compat(project: String, dep: String, json: bool) -> anyhow::Result<()> {
    let c = lwoodz::license::compatibility::check_compatibility(&project, &dep);
    let label = match c {
        lwoodz::license::compatibility::Compatibility::Compatible => "Compatible",
        lwoodz::license::compatibility::Compatibility::Incompatible => "Incompatible",
        lwoodz::license::compatibility::Compatibility::Warning => "Warning (review required)",
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "project": project,
                "dep": dep,
                "result": format!("{:?}", c),
                "label": label
            }))?
        );
    } else {
        println!("{} + {} => {} ({:?})", project, dep, label, c);
        if c == lwoodz::license::compatibility::Compatibility::Incompatible {
            println!(
                "  x Introducing '{}' into a '{}' project is likely incompatible for distribution.",
                dep, project
            );
        } else if c == lwoodz::license::compatibility::Compatibility::Warning {
            println!("  ! Check attribution / copyleft obligations.");
        } else {
            println!("  ok Compatible.");
        }
    }
    Ok(())
}

fn cmd_detect(path: std::path::PathBuf, json: bool) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(&path)?;
    let det = lwoodz::license::spdx::detect_spdx_from_text(&text);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "path": path.display().to_string(),
                "detected": det
            }))?
        );
    } else {
        match det {
            Some(id) => println!("{} => {}", path.display(), id),
            None => println!("{} => unknown (NOASSERTION)", path.display()),
        }
    }
    Ok(())
}

async fn cmd_explain(cfg: &lwoodz::config::Config, spdx: String) -> anyhow::Result<()> {
    if lwoodz::inference::is_available(&cfg.inference) {
        if let Some(text) = lwoodz::inference::groq::explain_license(&cfg.inference, &spdx).await {
            println!("{}", text);
            return Ok(());
        }
        println!("(Groq unavailable or failed -- falling back to local summary)\n");
    }
    if let Some(tpl) = lwoodz::license::templates::find_template(&spdx) {
        println!("{} ({})", tpl.name, tpl.spdx);
        println!(
            "  Copyleft: {}  Strong: {}",
            tpl.id.is_copyleft(),
            tpl.id.is_strong_copyleft()
        );
        println!(
            "  Body preview:\n{}",
            tpl.body.lines().take(12).collect::<Vec<_>>().join("\n")
        );
    } else {
        println!("Unknown SPDX id: {}", spdx);
        println!(
            "Known: {}",
            lwoodz::license::templates::all_licenses()
                .iter()
                .map(|t| t.spdx.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}

fn cmd_headers(cfg: &lwoodz::config::Config, dry_run: bool) -> anyhow::Result<()> {
    if dry_run {
        println!(
            "Dry run -- would process headers (spdx={}, holder={}, year={})",
            cfg.project.license, cfg.project.copyright_holder, cfg.project.copyright_year
        );
        println!("  Exclude: {:?}", cfg.headers.exclude);
        return Ok(());
    }
    let hc = lwoodz::license::header::HeaderConfig {
        holder: cfg.project.copyright_holder.clone(),
        year: cfg.project.copyright_year,
        spdx: lwoodz::license::spdx::normalize_spdx(&cfg.project.license),
        insert_spdx: cfg.headers.insert_spdx,
    };
    let results = lwoodz::license::header::ensure_headers(
        &cfg.repo_path,
        &hc,
        &cfg.headers.exclude,
        &cfg.headers.include,
    )?;
    let inserted = results
        .iter()
        .filter(|r| r.action == lwoodz::license::header::HeaderAction::Inserted)
        .count();
    let updated = results
        .iter()
        .filter(|r| r.action == lwoodz::license::header::HeaderAction::Updated)
        .count();
    let ok = results
        .iter()
        .filter(|r| r.action == lwoodz::license::header::HeaderAction::AlreadyPresent)
        .count();
    let skipped = results
        .iter()
        .filter(|r| r.action == lwoodz::license::header::HeaderAction::Skipped)
        .count();
    println!(
        "Headers: {} inserted, {} updated, {} already present, {} skipped ({} scanned)",
        inserted,
        updated,
        ok,
        skipped,
        results.len()
    );
    Ok(())
}

fn cmd_dual(cfg: &lwoodz::config::Config, license: String) -> anyhow::Result<()> {
    let p = lwoodz::generate::generate_dual_variant(cfg, &license)?;
    println!("Dual-license variant created at {}", p.display());
    Ok(())
}

fn cmd_status(cfg: &lwoodz::config::Config, json: bool) -> anyhow::Result<()> {
    let manifest = cfg.repo_path.join(".lwoodz/manifest.json");
    let audit_log = cfg.repo_path.join(&cfg.audit.log_path);

    let pid_path = lwoodz::daemon::pidfile::default_path(&cfg.repo_path);
    let daemon_pid = lwoodz::daemon::pidfile::read(&pid_path);
    let daemon_status = lwoodz::daemon::status::read(&cfg.repo_path);

    let info = serde_json::json!({
        "repo": cfg.repo_path.display().to_string(),
        "license": cfg.project.license,
        "mode": format!("{:?}", cfg.operation.mode),
        "manifest_exists": manifest.exists(),
        "audit_log_exists": audit_log.exists(),
        "daemon_pid": daemon_pid,
        "daemon_status": daemon_status,
    });
    if json {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("lwoodz status");
        println!("  Repo: {}", cfg.repo_path.display());
        println!(
            "  License: {}  Mode: {:?}",
            cfg.project.license, cfg.operation.mode
        );
        println!(
            "  Manifest: {}",
            if manifest.exists() {
                "present"
            } else {
                "missing"
            }
        );
        println!(
            "  Audit log: {}",
            if audit_log.exists() {
                "present"
            } else {
                "missing"
            }
        );
        if audit_log.exists() {
            if let Ok(t) = std::fs::read_to_string(&audit_log) {
                let lines = t.lines().count();
                println!("  Audit entries: {}", lines);
                if let Some(last) = t.lines().last() {
                    let preview: String = last.chars().take(200).collect();
                    println!("  Last: {}", preview);
                }
            }
        }
        match daemon_pid {
            Some(pid) => println!("  Daemon: running (pid {pid})"),
            None => println!("  Daemon: not running"),
        }
        if let Some(s) = daemon_status {
            println!("  Daemon started: {}", s.started_at);
            match (s.last_check_at, s.last_result, s.findings_count) {
                (Some(at), Some(result), Some(count)) => {
                    println!("  Last check: {at} — {result} ({count} findings)")
                }
                _ => println!("  Last check: none yet"),
            }
        }
    }
    Ok(())
}

fn print_audit(report: &lwoodz::audit::AuditReport) {
    println!(
        "lwoodz audit - {} ({})",
        report.project_license,
        if report.passed { "PASS" } else { "FAIL" }
    );
    println!("  Holder: {} ({})", report.holder, report.year);
    println!(
        "  LICENSE: {} (detected: {})",
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
        "  Deps: {} total ({} third-party, {} first-party), {} incompatible, {} warnings",
        report.compatibility.total_deps,
        report.compatibility.third_party,
        report.compatibility.first_party,
        report.compatibility.incompatible,
        report.compatibility.warnings
    );
    if let Some(hc) = &report.header_coverage {
        println!(
            "  Headers: {}/{} with copyright/SPDX",
            hc.with_header, hc.total_files
        );
    }
    if !report.findings.is_empty() {
        println!("\nFindings ({}):", report.findings.len());
        for f in &report.findings {
            let icon = match f.level.as_str() {
                "error" => "x",
                "warning" => "!",
                _ => ".",
            };
            println!("  {} [{}] {} - {}", icon, f.level, f.code, f.message);
        }
    } else {
        println!("\n  No findings -- repository is compliant.");
    }
}
