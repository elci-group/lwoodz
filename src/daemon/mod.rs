// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT

use crate::config::Config;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub mod pidfile;
pub mod status;
pub mod watcher;

use status::DaemonStatus;

pub struct Daemon {
    pub config: Arc<RwLock<Config>>,
}

impl Daemon {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let cfg = self.config.read().await.clone();

        if cfg.daemon.startup_guard {
            let status = std::process::Command::new("git")
                .args(["status", "--porcelain"])
                .output();
            if let Ok(out) = status {
                if !out.stdout.is_empty() {
                    anyhow::bail!(
                        "startup_guard: worktree has uncommitted changes; refusing to start (--force to override)"
                    );
                }
            }
        }

        let pid_path = pidfile::default_path(&cfg.repo_path);
        pidfile::acquire(&pid_path)?;

        tracing::info!(
            repo = %cfg.repo_path.display(),
            license = %cfg.project.license,
            mode = ?cfg.operation.mode,
            pid = std::process::id(),
            "lwoodz daemon starting (legal state governor)"
        );

        let mut initial_status = DaemonStatus::starting(&cfg);
        if let Some((passed, findings)) = initial_sync(&cfg) {
            initial_status.record_check(passed, findings);
        }
        let _ = initial_status.write(&cfg.repo_path);
        let shared_status = Arc::new(RwLock::new(initial_status));

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());
        let watch_task = tokio::spawn(watcher::watch(
            cfg.clone(),
            self.config.clone(),
            shutdown_rx,
            shared_status,
        ));

        shutdown_signal().await;
        let grace = Duration::from_secs(cfg.daemon.shutdown_grace_secs);
        tracing::info!(
            grace_secs = grace.as_secs(),
            "shutdown signal received, stopping daemon"
        );
        // The watcher only observes this between cycles, never mid-audit, so
        // an in-flight license check always finishes rather than being cut
        // off half-written.
        let _ = shutdown_tx.send(());

        match tokio::time::timeout(grace, watch_task).await {
            Ok(Ok(Ok(()))) => tracing::info!("watcher stopped cleanly"),
            Ok(Ok(Err(e))) => tracing::warn!(error = %e, "watcher exited with error"),
            Ok(Err(e)) => tracing::warn!(error = %e, "watcher task panicked"),
            Err(_) => tracing::warn!(
                grace_secs = grace.as_secs(),
                "watcher did not stop within grace period; exiting anyway"
            ),
        }

        pidfile::release(&pid_path);
        tracing::info!("lwoodz daemon stopped");
        Ok(())
    }
}

/// Waits for SIGINT (Ctrl+C, portable) or SIGTERM (Unix only — this is what
/// `systemctl stop` / `kill` send by default).
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        match signal(SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received SIGINT"),
        _ = terminate => tracing::info!("received SIGTERM"),
    }
}

/// Returns `Some((passed, findings_count))` when an audit actually ran
/// (observe mode), so the caller can seed the daemon's status surface.
fn initial_sync(cfg: &Config) -> Option<(bool, usize)> {
    if cfg.operation.mode != crate::config::loader::OperationMode::Enforce {
        tracing::info!("operation.mode=observe — skipping file generation (dry run only)");
        return match crate::audit::run(cfg) {
            Ok(report) => {
                if !report.passed {
                    tracing::warn!(
                        findings = report.findings.len(),
                        "audit found issues (observe mode — not enforcing)"
                    );
                    for f in &report.findings {
                        tracing::warn!(level = %f.level, code = %f.code, msg = %f.message);
                    }
                }
                Some((report.passed, report.findings.len()))
            }
            Err(e) => {
                tracing::warn!(error = %e, "initial audit failed");
                None
            }
        };
    }

    match run_enforce_sync(cfg) {
        Ok(()) => None,
        Err(e) => {
            tracing::warn!(error = %e, "initial sync failed");
            None
        }
    }
}

fn run_enforce_sync(cfg: &Config) -> anyhow::Result<()> {
    let res = crate::generate::generate_license_file(cfg)?;
    tracing::info!(
        license = %res.spdx,
        path = %res.license_path.display(),
        "initial license sync complete"
    );

    if cfg.headers.enabled {
        let hc = crate::license::header::HeaderConfig {
            holder: cfg.project.copyright_holder.clone(),
            year: cfg.project.copyright_year,
            spdx: crate::license::spdx::normalize_spdx(&cfg.project.license),
            insert_spdx: cfg.headers.insert_spdx,
        };
        let results = crate::license::header::ensure_headers(
            &cfg.repo_path,
            &hc,
            &cfg.headers.exclude,
            &cfg.headers.include,
        )?;
        let inserted = results
            .iter()
            .filter(|r| r.action == crate::license::header::HeaderAction::Inserted)
            .count();
        let updated = results
            .iter()
            .filter(|r| r.action == crate::license::header::HeaderAction::Updated)
            .count();
        tracing::info!(
            inserted,
            updated,
            total = results.len(),
            "header sync complete"
        );
    }

    Ok(())
}
