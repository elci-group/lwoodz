# Changelog

All notable changes to Lwoodz are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Version control: project brought under git for the first time.
- CI pipeline (`.github/workflows/lwoodz-ci.yml`): fmt, clippy, `cargo audit`,
  `cargo deny`, build, and test on every push and pull request.
- `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `deny.toml`.
- Own source tree now carries the SPDX headers it inserts into other
  projects (`lwoodz-cli headers`, 20 files newly headered, 7 updated).
- **Real Cargo license resolution.** Replaced the 15-crate hardcoded
  whitelist with `cargo metadata`, which returns each resolved package's own
  declared `license` straight from that crate's `Cargo.toml`. Falls back to
  an unresolved manifest-only scan if `cargo` can't run at all, rather than
  guessing.
- **Real npm license resolution.** `package-lock.json` (v2/v3) already
  records each resolved package's declared `license`; scanning now reads it
  directly, including transitive dependencies that never appear in
  `package.json`. Falls back to the installed package's own `package.json`
  (or a `LICENSE` file) under `node_modules/` when the lockfile is absent or
  incomplete.
- **Real Python license resolution.** When a `.venv`/`venv`/`env` is
  discoverable, scanning now reads each package's own
  `*.dist-info/METADATA` — the `License:` field, or the
  `Classifier: License :: OSI Approved :: …` Trove classifier as a fallback.
- **Daemon lifecycle.** `--daemon` now writes `.lwoodz/daemon.pid` and
  refuses to start a second instance against the same repo while one is
  running (with stale-pid detection for a crashed prior instance); handles
  `SIGTERM`/`SIGINT` gracefully, always letting an in-flight check finish
  before stopping, bounded by the existing (previously unused)
  `[daemon] shutdown_grace_secs`; and writes live status to
  `.lwoodz/status.json` on start and after every watch cycle.
  `lwoodz-cli status` now reports whether the daemon is running and the
  result of its last check.
- Hardened systemd unit template at `contrib/systemd/lwoodz.service`,
  documented in the README.
- Groq requests now retry transient failures (network errors, 5xx, 429) with
  exponential backoff, and never retry a 4xx (retrying a bad API key just
  wastes the timeout). `inference::advise` also now actually checks
  `[inference] provider` instead of always calling Groq regardless of what
  was configured.
- `cargo-llvm-cov` coverage floor in CI (35%, measured at 41.6% when added).
- **Real Go license resolution**, closing the gap noted above: reads
  `go env GOMODCACHE` and, for each `go.mod` requirement, looks for that
  module's own `LICENSE`/`COPYING` file in the local module cache. Same
  "ask the artifact" approach as the other three ecosystems — Cargo, npm,
  Python, and now Go all resolve real licenses, not guesses.
- **Scanner trait** (`src/manifest/mod.rs`): the four ecosystem scanners are
  now trait implementations registered in one place, so adding a new
  ecosystem (Maven, Gradle, NuGet, RubyGems) is additive — implement the
  trait, add one line to the registry — instead of editing `scan()` itself.
  See the new "Adding a new ecosystem" README section.
- `src/cli.rs`: both binaries' `clap` definitions now live in one place in
  the library, so `examples/gen-man.rs` and `lwoodz-cli completions` are
  generated from the real, current CLI surface instead of hand-authored
  docs that can drift from it. Real man pages committed at `man/lwoodz.1`
  and `man/lwoodz-cli.1`; `lwoodz-cli completions <shell>` added
  (bash/zsh/fish/elvish/powershell).
- Config precedence (CLI flag > env var > config file > defaults) and the
  two-binary split's rationale documented in the README.
- Fuzz targets (`fuzz/`, standalone `cargo-fuzz` workspace) for the three
  hand-written parsers that run against input lwoodz doesn't control: SPDX
  text detection (now reading arbitrary `LICENSE` files from every
  ecosystem's dependency tree), header insert/update/strip, and `go.mod`
  parsing. `license::header::apply_header` and `manifest::go::parse_go_mod`
  were pulled out as pure functions (no disk I/O) specifically so they're
  fuzzable directly.
- Dormant release workflow (`.github/workflows/lwoodz-release.yml`):
  pushing a `lwoodz-v*` tag builds unsigned Linux/macOS/Windows archives and
  opens a **draft** GitHub Release for human review. No signing yet
  (unlike `scotia`'s minisign-based installers) — see the README's
  "Releases" section for the honest state of this.

### Known gaps (tracked for [Unreleased] follow-up)

- License resolution for all four ecosystems depends on a locally installed
  environment (`cargo metadata`'s registry cache, `node_modules`, a Python
  venv, or `GOMODCACHE`) or a lockfile that already carries license data. A
  manifest with no lockfile and nothing installed/downloaded still returns
  unresolved licenses — there is intentionally no network call to a
  registry API yet, for any of the four ecosystems.
- The two-binary CLI surface (`lwoodz`, `lwoodz-cli`) was deliberately *not*
  merged this round — see README's CLI reference for why the split exists.
  Revisit only with explicit sign-off, since it's a breaking change for
  anyone already scripting against either binary.
- Inference is still single-provider (Groq only) — `advise()` fails closed
  on an unrecognized provider instead of silently misusing Groq's endpoint,
  but there's no second implementation yet.
- No response caching for repeated `explain` calls — low value today since
  each CLI invocation is a fresh process with nothing to cache across; would
  matter more if the daemon starts calling inference directly.
- Release archives are unsigned; no installer script yet.
