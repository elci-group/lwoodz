// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
use crate::config::Config;
use crate::daemon::status::DaemonStatus;
use notify::Watcher;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub async fn watch(
    initial_cfg: Config,
    shared: Arc<RwLock<Config>>,
    mut shutdown: tokio::sync::watch::Receiver<()>,
    status: Arc<RwLock<DaemonStatus>>,
) -> anyhow::Result<()> {
    let watch_path = initial_cfg.watch.path.clone();
    let repo_root = initial_cfg.repo_path.clone();
    let watch_root = if std::path::Path::new(&watch_path).is_absolute() {
        std::path::PathBuf::from(&watch_path)
    } else {
        repo_root.join(&watch_path)
    };

    tracing::info!(path = %watch_root.display(), "watching for manifest changes");

    let (tx, mut rx) = tokio::sync::mpsc::channel::<notify::Result<notify::Event>>(64);

    let mut watcher = notify::RecommendedWatcher::new(
        move |res| {
            let _ = tx.blocking_send(res);
        },
        notify::Config::default().with_poll_interval(Duration::from_secs(2)),
    )?;

    watcher.watch(&watch_root, notify::RecursiveMode::Recursive)?;

    let mut debounce = tokio::time::interval(Duration::from_secs(3));
    let mut pending = false;

    loop {
        tokio::select! {
            _ = debounce.tick() => {
                if pending {
                    pending = false;
                    let cfg = shared.read().await.clone();
                    match on_change(&cfg).await {
                        Ok((passed, findings)) => {
                            status.write().await.record_check(passed, findings);
                            if let Err(e) = status.read().await.write(&cfg.repo_path) {
                                tracing::warn!(error = %e, "failed to persist daemon status");
                            }
                        }
                        Err(e) => tracing::warn!(error = %e, "change handler failed"),
                    }
                }
            }
            ev = rx.recv() => {
                match ev {
                    Some(Ok(event)) => {
                        if is_relevant(&event) {
                            tracing::debug!(?event, "relevant change detected");
                            pending = true;
                        }
                    }
                    Some(Err(e)) => tracing::warn!(error = %e, "watch error"),
                    None => break,
                }
            }
            // Only observed between cycles — never mid-`on_change` — so a
            // shutdown request can't interrupt a check that's already
            // writing files.
            _ = shutdown.changed() => {
                tracing::info!("watcher received shutdown signal, stopping");
                break;
            }
        }
    }

    Ok(())
}

fn is_relevant(event: &notify::Event) -> bool {
    // Only care about creates/modifies on manifest-like files
    let relevant_names = [
        "Cargo.toml",
        "Cargo.lock",
        "package.json",
        "package-lock.json",
        "pyproject.toml",
        "requirements.txt",
        "go.mod",
        "go.sum",
        "lwoodz.toml",
        ".lwoodz.toml",
    ];
    for path in &event.paths {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if relevant_names.contains(&name) {
            return true;
        }
        // Any lwoodz.toml change is relevant
        if path.to_string_lossy().contains("lwoodz.toml") {
            return true;
        }
    }
    // Also trigger on any file if kind is relevant? We limit to manifest files to avoid noise.
    false
}

/// Runs one check cycle and returns `(passed, findings_count)` for the
/// status surface.
async fn on_change(cfg: &Config) -> anyhow::Result<(bool, usize)> {
    tracing::info!("manifest change detected — re-auditing");

    // Re-scan manifests and check compatibility
    let report = crate::audit::run(cfg)?;
    if !report.passed {
        tracing::warn!(
            findings = report.findings.len(),
            incompatible = report.compatibility.incompatible,
            "licensing issues detected"
        );
        for f in &report.findings {
            if f.level == "error" {
                tracing::error!(code = %f.code, msg = %f.message);
            } else {
                tracing::warn!(code = %f.code, msg = %f.message);
            }
        }
        if cfg.compatibility.enforce && report.compatibility.incompatible > 0 {
            tracing::error!(
                "compatibility.enforce=true and incompatible deps found — advise review before release"
            );
        }
    } else {
        tracing::info!("audit passed — no blocking issues");
    }

    // In enforce mode, refresh attribution + manifest
    if cfg.operation.mode == crate::config::loader::OperationMode::Enforce {
        let _ = crate::generate::generate_license_file(cfg)?;
        tracing::info!("regenerated licensing artifacts after change");
    }

    Ok((report.passed, report.findings.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_relevant_matches_known_manifest_files() {
        let event = notify::Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Any),
            paths: vec![std::path::PathBuf::from("/repo/Cargo.toml")],
            attrs: Default::default(),
        };
        assert!(is_relevant(&event));
    }

    #[test]
    fn is_relevant_ignores_unrelated_files() {
        let event = notify::Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Any),
            paths: vec![std::path::PathBuf::from("/repo/src/main.rs")],
            attrs: Default::default(),
        };
        assert!(!is_relevant(&event));
    }

    #[tokio::test]
    async fn watch_stops_promptly_on_shutdown_signal() {
        let dir = tempfile::tempdir().unwrap();
        // WatchConfig already defaults path to ".", so only repo_path needs overriding.
        let cfg = Config {
            repo_path: dir.path().to_path_buf(),
            ..Config::default()
        };

        let shared = Arc::new(RwLock::new(cfg.clone()));
        let status = Arc::new(RwLock::new(DaemonStatus::starting(&cfg)));
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(());

        let task = tokio::spawn(watch(cfg, shared, shutdown_rx, status));
        tokio::time::sleep(Duration::from_millis(50)).await;
        shutdown_tx.send(()).unwrap();

        let result = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .expect("watch task should exit promptly after shutdown signal");
        assert!(result.is_ok(), "watch task should not panic");
        assert!(
            result.unwrap().is_ok(),
            "watch should return Ok on clean shutdown"
        );
    }
}
