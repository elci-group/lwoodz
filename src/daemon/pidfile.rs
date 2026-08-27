// Copyright (c) 2026 sal
// SPDX-License-Identifier: MIT
#![allow(unused_variables)] // Legacy tracing field bindings are stringified by telemetry.
use crate::telemetry as tracing;
use std::path::{Path, PathBuf};

/// Default location for the daemon's PID file, relative to the repo root —
/// mirrors kaptaind's own `.kaptaind/daemon.pid` convention.
pub fn default_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".lwoodz").join("daemon.pid")
}

/// Writes the current process's PID, refusing to clobber a PID file that
/// still points at a live process (a second daemon started against the same
/// repo, most likely by accident).
pub fn acquire(path: &Path) -> anyhow::Result<()> {
    if let Some(existing) = read(path) {
        if is_running(existing) {
            anyhow::bail!(
                "daemon already running (pid {existing}, pid file {}) — stop it first or remove a stale pid file",
                path.display()
            );
        }
        tracing::warn!(
            stale_pid = existing,
            path = %path.display(),
            "removing stale pid file (process no longer running)"
        );
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, std::process::id().to_string())?;
    Ok(())
}

/// Removes the PID file, but only if it still names this process — avoids
/// deleting a pid file legitimately written by a newer daemon instance that
/// started while a previous one was already exiting.
pub fn release(path: &Path) {
    if read(path) == Some(std::process::id()) {
        let _ = std::fs::remove_file(path);
    }
}

pub fn read(path: &Path) -> Option<u32> {
    std::fs::read_to_string(path)
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()
}

/// Best-effort liveness check. On Linux this is exact (`/proc/<pid>`); on
/// platforms without a `/proc` filesystem it conservatively assumes the
/// process is still running rather than risk a false "stale" report.
fn is_running(pid: u32) -> bool {
    let proc_path = PathBuf::from(format!("/proc/{pid}"));
    if proc_path.exists() {
        return true;
    }
    // No /proc (e.g. macOS) — /proc never existing is not evidence of
    // staleness, so don't claim the pid is dead.
    !PathBuf::from("/proc").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_then_release_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("daemon.pid");
        acquire(&path).unwrap();
        assert_eq!(read(&path), Some(std::process::id()));
        release(&path);
        assert!(!path.exists());
    }

    #[test]
    fn acquire_refuses_when_pid_file_names_a_live_process() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("daemon.pid");
        // Our own pid is definitely "running".
        std::fs::write(&path, std::process::id().to_string()).unwrap();
        let err = acquire(&path).unwrap_err();
        assert!(err.to_string().contains("already running"));
    }

    #[test]
    fn acquire_overwrites_a_stale_pid_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("daemon.pid");
        // PID 1 is init/systemd on any real Linux box, never this test
        // binary — but we only assert this exercises the "not us" path by
        // using a value guaranteed unequal to our own pid.
        let fake_dead_pid = if std::process::id() == 999_999 {
            999_998
        } else {
            999_999
        };
        std::fs::write(&path, fake_dead_pid.to_string()).unwrap();
        // This assertion only holds on Linux (/proc present); skip elsewhere.
        if PathBuf::from("/proc").exists() {
            acquire(&path).unwrap();
            assert_eq!(read(&path), Some(std::process::id()));
        }
    }

    #[test]
    fn release_does_not_delete_a_pid_file_owned_by_another_process() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("daemon.pid");
        std::fs::write(&path, "424242").unwrap();
        release(&path);
        assert!(path.exists(), "release must not touch a pid it doesn't own");
    }
}
