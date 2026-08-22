# Lwoodz

> **Lwoodz is to license document generation and maintenance what Kaptaind is to version management.**

The canonical licensing daemon for a repository — governs legal state the way Kaptaind governs temporal state.

| Kaptaind                        | Lwoodz                                                    |
| ------------------------------- | --------------------------------------------------------- |
| Manages project versions        | Manages project licenses                                  |
| Creates and updates `VERSION`   | Creates and updates `LICENSE` and related legal documents |
| Tracks release lifecycle        | Tracks licensing lifecycle                                |
| Ensures version consistency     | Ensures license consistency                               |
| Automates semantic versioning   | Automates legal document generation and maintenance       |
| Integrates with release tooling | Integrates with packaging, repositories, and distribution |

## What it does

- **Detects dependencies** from `Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod` (and `Cargo.lock`) and resolves their SPDX licenses.
- **Checks compatibility** — warns when introducing incompatible dependencies (e.g. GPL into an MIT project) via a conservative FSF/OSI-derived matrix.
- **Generates** `LICENSE`, `NOTICE`, `COPYRIGHT`, `THIRD_PARTY_NOTICES` and a machine-readable SPDX manifest (`.lwoodz/manifest.json`, SPDX-2.3).
- **Headers** — inserts or updates `Copyright (c) YEAR HOLDER` + `SPDX-License-Identifier` in source files (Rust, Go, Python, JS/TS, C/C++, Java, etc.), preserving shebangs.
- **SPDX** — normalizes aliases (`Apache 2.0` -> `Apache-2.0`), detects license from existing `LICENSE` text, validates identifiers.
- **Dual / commercial variants** — `lwoodz-cli dual Apache-2.0` produces a dual-licensed file.
- **Audit** — pre-release gate: license file presence, SPDX validity, dependency compatibility, header coverage. Machine-readable JSON + `audit.jsonl` log.
- **Contributors / DCO** — maintains `.lwoodz/contributors.json`.
- **Daemon** — `lwoodz --daemon` watches manifests and keeps legal state continuously consistent (observe vs enforce).
- **Inference** — optional AI-assisted advice via `GROQ_API_KEY` (Groq OpenAI-compatible API, `llama-3.3-70b-versatile` default).

## Install

```bash
cargo install --path .
# or
cargo build --release
sudo cp target/release/lwoodz target/release/lwoodz-cli /usr/local/bin/
```

## Quick start

```bash
# 1. Initialize config
lwoodz --init
# or
lwoodz-cli init

# 2. Edit lwoodz.toml
#    [project] license = "MIT"  (or Apache-2.0, GPL-3.0-only, etc.)
#    [project] copyright_holder = "Acme Inc"
#    [project] copyright_year = 2026

# 3. Generate legal docs
lwoodz --generate
lwoodz-cli generate

# 4. Audit
lwoodz --audit
lwoodz --check          # compatibility only
lwoodz --audit --json
lwoodz-cli --json audit # note: --json is global, goes before the subcommand

# 5. Headers
lwoodz-cli headers              # insert/update headers (enforce mode also does this)
lwoodz-cli headers --dry-run

# 6. Daemon (watch mode)
lwoodz --daemon
# Check status (also reports whether the daemon is running and its last check)
lwoodz-cli status
```

## Daemon lifecycle

`lwoodz --daemon` runs in the foreground (systemd manages backgrounding —
there's no double-fork). It:

- Writes a PID file to `.lwoodz/daemon.pid` and refuses to start a second
  instance against the same repo while one is already running; a PID file
  left behind by a crashed process is detected as stale and reclaimed.
- Handles `SIGTERM` (what `systemctl stop` / `kill` send) and `SIGINT`
  (Ctrl+C) gracefully: it finishes any check currently in progress — never
  cuts one off mid-write — then stops watching and exits. `[daemon]
  shutdown_grace_secs` in `lwoodz.toml` bounds how long it will wait before
  giving up and exiting anyway.
- Writes live status to `.lwoodz/status.json` (pid, last check time,
  pass/fail, findings count) on start and after every watch cycle —
  `lwoodz-cli status` reads it, no need to query the running process.

### Running under systemd

A hardened unit template is at `contrib/systemd/lwoodz.service`:

```bash
sudo cp contrib/systemd/lwoodz.service /etc/systemd/system/lwoodz@.service
sudo systemctl daemon-reload
sudo systemctl enable --now lwoodz@$(systemd-escape /path/to/your/repo)
journalctl -u 'lwoodz@*' -f
```

## Configuration (`lwoodz.toml`)

Generate with `lwoodz --init`. Key sections:

```toml
[project]
license = "MIT"                       # SPDX id
copyright_holder = "Your Name"
copyright_year = 2026
project_name = "myproject"
# dual_license = "Apache-2.0"
# commercial_license = "SEE LICENSE IN Commercial-LICENSE"

[operation]
mode = "observe"                      # observe (report) | enforce (write)

[generate]
license_file = "LICENSE"
notice_file = "NOTICE"
copyright_file = "COPYRIGHT"
attribution_file = "THIRD_PARTY_NOTICES"
overwrite = true

[headers]
enabled = true
insert_spdx = true
exclude = ["target/**", "node_modules/**", "dist/**", ".git/**", ".lwoodz/**"]

[compatibility]
enforce = true
policy = "permissive"                 # permissive | copyleft-allowed | strict

[spdx]
produce_manifest = true
manifest_path = ".lwoodz/manifest.json"

[audit]
log_path = ".lwoodz/audit.jsonl"
fail_on_incompatible = false

[inference]
enabled = true
provider = "groq"
model = "auto"
groq_model = "llama-3.3-70b-versatile"  # GROQ_API_KEY must be set

[daemon]
startup_guard = false

[watch]
path = "."
recursive = true
```

### Configuration precedence

Highest to lowest, each layer overriding the one below it for the fields it sets:

1. **CLI flags** on the invocation itself — `--config <path>` selects which file loads at all.
2. **Environment variables** — `LWOODZ_LICENSE`, `LWOODZ_HOLDER` override the loaded file's `[project]` values; `LWOODZ_CONFIG` overrides *which* file `find_config_path()` picks, taking priority even over the `lwoodz.toml` / `.lwoodz.toml` / `.lwoodz/config.toml` search order below.
3. **The config file itself** — resolved by `LWOODZ_CONFIG` if set, else the first of `lwoodz.toml`, `.lwoodz.toml`, `.lwoodz/config.toml` found by walking from the current directory up to the filesystem root.
4. **Built-in defaults** (`Config::default()`) — used for any field the file omits; every section is optional in TOML via `#[serde(default)]`, so a minimal `[project]`-only file is valid.

`GROQ_API_KEY` is the one exception: it's read directly from the environment at call time in `src/inference/groq.rs`, never from the config file, and never overridable by a `[project]`/`[inference]` value — a licensing tool shouldn't make it easy to accidentally commit an API key to `lwoodz.toml`.

Environment overrides: `LWOODZ_CONFIG`, `LWOODZ_LICENSE`, `LWOODZ_HOLDER`, `GROQ_API_KEY`, `RUST_LOG`.

## CLI reference

Lwoodz ships two binaries with a deliberate split, not overlapping accidents:

- **`lwoodz`** is the thing you point a process supervisor at — `lwoodz --daemon`
  runs in the foreground (see [Daemon lifecycle](#daemon-lifecycle)), and its
  non-daemon flags (`--audit`, `--generate`, ...) exist so a single static
  binary works both as the long-running watcher and as the one-shot check a
  pre-commit hook or CI step calls.
- **`lwoodz-cli`** is the thing a human or a script runs interactively —
  every operation as its own subcommand (`explain`, `compat`, `dual`,
  `completions`, ...), the shape you'd expect from a normal CLI tool, and
  the one that grows as new one-off commands get added.

Both read the same `lwoodz.toml` and share the same library code
(`src/cli.rs` defines both argument sets from a single source, so their help
text and man pages can't drift from what's actually implemented) — this is
a deliberate two-binary split, not two versions of the same idea.

**`lwoodz` (daemon + audit)**

```
lwoodz              # audit (default)
lwoodz --daemon     # watch manifests continuously
lwoodz --generate   # generate LICENSE/NOTICE/COPYRIGHT/attribution/manifest
lwoodz --audit      # full audit
lwoodz --check      # compat check only
lwoodz --init       # create lwoodz.toml
lwoodz --json       # JSON output (audit/check)
```

**`lwoodz-cli`**

```
lwoodz-cli init [--force]
lwoodz-cli generate [--dry-run]
lwoodz-cli audit
lwoodz-cli check [--strict]
lwoodz-cli licenses
lwoodz-cli compat <project-spdx> <dep-spdx>
lwoodz-cli detect [path]
lwoodz-cli explain <spdx>          # uses GROQ_API_KEY if available
lwoodz-cli headers [--dry-run]
lwoodz-cli dual <spdx>
lwoodz-cli status
lwoodz-cli version
lwoodz-cli completions <bash|zsh|fish|elvish|powershell>
```

Man pages for both binaries live in `man/` (regenerate with
`cargo run --example gen-man` after changing `src/cli.rs`).

## Adding a new ecosystem

Dependency scanning is a small [`Scanner`
trait](src/manifest/mod.rs) (`name()` + `scan()`), with one implementation
per ecosystem (`cargo.rs`, `npm.rs`, `python.rs`, `go.rs`) registered in
`manifest::scanners()`. Adding Maven, Gradle, NuGet, or RubyGems support
means implementing the trait for a new module and adding one line to that
registry — `manifest::scan()` and every caller of it (`audit`, `generate`,
`lwoodz-cli check`/`detect`) never need to change. Follow the existing
pattern: resolve licenses from something the ecosystem's own tooling
produces (a lockfile field, an installed package's own metadata, a real
`LICENSE` file) rather than a hardcoded guess — see the `manifest/*.rs`
module docs for what each scanner actually reads.

## Fuzzing

`fuzz/` (a standalone `cargo-fuzz` workspace, so it never affects normal
`cargo build`/`test`/`clippy`) targets the hand-written parsers that run
against untrusted input — other people's `LICENSE` files, `go.mod` files,
and source files being headered:

```bash
cargo install cargo-fuzz
cargo +nightly fuzz run fuzz_spdx_detect    # SPDX text/expression parsing
cargo +nightly fuzz run fuzz_header_apply   # header insert/update/strip
cargo +nightly fuzz run fuzz_go_mod_parse   # go.mod require-block parsing
```

## Releases

Pushing a `lwoodz-v*` tag builds unsigned Linux/macOS/Windows archives
(binaries + LICENSE/NOTICE/COPYRIGHT/README + man pages) and opens a
**draft** GitHub Release for human review — nothing publishes
unattended. See `.github/workflows/lwoodz-release.yml`. There's no
installer or signing yet (contrast with `scotia`'s minisign-signed,
per-platform installers) — for now, `cargo install --path .` or the
release archive are the supported install paths.

## GROQ inference

Lwoodz uses `GROQ_API_KEY` for AI-assisted licensing advice — the same pattern as Kaptaind's inference routing, but pinned to Groq.

```
export GROQ_API_KEY="gsk_..."
lwoodz-cli explain MIT
# -> Groq-powered explanation, or local fallback if no key
```

No key is required for core functionality; local templates, SPDX validation, and compatibility checks work offline.

## Supported licenses (SPDX)

`MIT`, `Apache-2.0`, `GPL-2.0-only`, `GPL-3.0-only`, `LGPL-2.1-only`, `LGPL-3.0-only`, `AGPL-3.0-only`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `MPL-2.0`, `Unlicense`, `CC0-1.0`, `BSL-1.0`, `Zlib` (+ aliases and compound `AND`/`OR`).

## License

MIT — see `LICENSE`.
# lwoodz
