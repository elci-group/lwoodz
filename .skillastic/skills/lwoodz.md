---
name: lwoodz
description: >-
  Use `lwoodz` to manage license documents, SPDX metadata, source-file headers,
  and dependency license compatibility for a repository. Reach for it whenever
  LICENSE, NOTICE, COPYRIGHT, THIRD_PARTY_NOTICES, or SPDX headers need to be
  created, updated, audited, or kept consistent. Prefer `lwoodz-cli` for
  interactive one-off tasks and `lwoodz --daemon` for continuous enforcement.
---

# lwoodz

`lwoodz` is the canonical licensing daemon for a repository. It detects
dependencies, resolves SPDX licenses, checks compatibility, generates legal
documents, inserts/maintains source headers, and can run continuously to keep
legal state consistent.

## Quick decision flow

1. **First time in a repo?** `lwoodz-cli init` then edit `lwoodz.toml`.
2. **Need legal docs?** `lwoodz-cli generate` (or `lwoodz --generate`).
3. **Want a compatibility/audit gate?** `lwoodz --audit` or `lwoodz-cli audit`.
4. **Need source headers updated?** `lwoodz-cli headers [--dry-run]`.
5. **Running CI/pre-commit?** `lwoodz --audit --json`.
6. **Continuous enforcement?** `lwoodz --daemon`.

## When to call lwoodz

Call lwoodz when you need to:

- Initialize or update `LICENSE`, `NOTICE`, `COPYRIGHT`, `THIRD_PARTY_NOTICES`.
- Produce or refresh the SPDX manifest (`.lwoodz/manifest.json`).
- Insert or update `SPDX-License-Identifier` + copyright headers in source files.
- Audit dependency licenses for compatibility with the project's license.
- Check whether a dependency's SPDX identifier is compatible with the project.
- Explain a license (with optional `GROQ_API_KEY`).
- Keep legal state consistent in watch mode.

Do **not** call lwoodz when:

- The task is unrelated to licensing, headers, attribution, or legal documents.
- You have not yet configured `[project]` in `lwoodz.toml` (run `init` first).
- You intend to write headers without a dry-run in a large, unfamiliar repo.

## Two binaries

| Binary | Use for |
|--------|---------|
| `lwoodz` | One-shot daemon-style operations and CI: `--daemon`, `--generate`, `--audit`, `--check`, `--init`, `--json`. |
| `lwoodz-cli` | Human/script-friendly subcommands: `init`, `generate`, `audit`, `check`, `headers`, `compat`, `explain`, `status`, ... |

Both read the same `lwoodz.toml` and share library code. Use `lwoodz-cli` unless
the surrounding context specifically asks for daemon/JSON output.

## CLI at a glance

### `lwoodz` (daemon + audit)

```text
lwoodz              # audit (default)
lwoodz --daemon     # watch manifests continuously
lwoodz --generate   # generate legal docs and manifest
lwoodz --audit      # full audit
lwoodz --check      # compatibility check only
lwoodz --init       # create lwoodz.toml
lwoodz --json       # JSON output (audit/check)
```

### `lwoodz-cli`

```text
lwoodz-cli init [--force]
lwoodz-cli generate [--dry-run]
lwoodz-cli audit
lwoodz-cli check [--strict]
lwoodz-cli licenses
lwoodz-cli compat <project-spdx> <dep-spdx>
lwoodz-cli detect [path]
lwoodz-cli explain <spdx>
lwoodz-cli headers [--dry-run]
lwoodz-cli dual <spdx>
lwoodz-cli status
lwoodz-cli version
lwoodz-cli completions <bash|zsh|fish|elvish|powershell>
```

## Configuration

Generate with `lwoodz --init` or `lwoodz-cli init`. Key sections:

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

Highest to lowest:

1. CLI flags, including `--config <path>`.
2. Environment variables: `LWOODZ_LICENSE`, `LWOODZ_HOLDER`, `LWOODZ_CONFIG`.
3. The config file (`lwoodz.toml`, `.lwoodz.toml`, or `.lwoodz/config.toml`).
4. Built-in defaults.

`GROQ_API_KEY` is read directly from the environment at call time and never from
the config file.

## Common commands

```bash
# Initialize config
cd /path/to/repo
lwoodz-cli init

# Generate/update LICENSE, NOTICE, COPYRIGHT, THIRD_PARTY_NOTICES, manifest
lwoodz-cli generate

# Audit everything and emit JSON for CI
lwoodz --audit --json

# Compatibility check only
lwoodz --check
lwoodz-cli check

# Update source headers (dry-run first)
lwoodz-cli headers --dry-run
lwoodz-cli headers

# Explain a license (uses GROQ_API_KEY if available)
lwoodz-cli explain MIT

# Check whether a dependency license is compatible with the project
lwoodz-cli compat MIT GPL-3.0-only

# Start the daemon
lwoodz --daemon

# Check daemon status
lwoodz-cli status
```

## Agent workflows

### Bootstrap legal state in a new repo

1. `lwoodz-cli init`
2. Edit `lwoodz.toml`: set `[project] license`, `copyright_holder`, `copyright_year`.
3. `lwoodz-cli generate` to create legal docs and `.lwoodz/manifest.json`.
4. `lwoodz-cli headers --dry-run` to preview header changes.
5. `lwoodz-cli headers` to apply them.
6. `lwoodz --audit` to confirm everything passes.

### CI/pre-commit gate

```bash
lwoodz --audit --json
```

Inspect exit code and the JSON report. Do not declare the change complete while
the audit reports incompatible dependencies, missing headers, or missing license
files.

### Dependency license check

After adding a dependency:

```bash
lwoodz-cli check
# or
lwoodz-cli compat <project-spdx> <dep-spdx>
```

If the new dependency is incompatible with the project license, resolve it
before committing (change dependency, add exception, or reconsider license).

### Daemon deployment

For continuous enforcement under a supervisor:

```bash
lwoodz --daemon
```

Or use the systemd unit template in `contrib/systemd/lwoodz.service`.

The daemon writes status to `.lwoodz/status.json`; `lwoodz-cli status` reads it
without needing to query the process.

## Supported ecosystems

Dependency scanning supports:

- Rust (`Cargo.toml`, `Cargo.lock`)
- Node.js/npm (`package.json`)
- Python (`pyproject.toml`)
- Go (`go.mod`)

Adding another ecosystem means implementing the `Scanner` trait in
`src/manifest/` and registering it in `manifest::scanners()`.

## Supported licenses (SPDX)

`MIT`, `Apache-2.0`, `GPL-2.0-only`, `GPL-3.0-only`, `LGPL-2.1-only`,
`LGPL-3.0-only`, `AGPL-3.0-only`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`,
`MPL-2.0`, `Unlicense`, `CC0-1.0`, `BSL-1.0`, `Zlib`, plus aliases and compound
`AND`/`OR` expressions.

## Anti-patterns

- **Running `headers` without `--dry-run` first** in a repo you don't fully
  understand. Headers modify many files.
- **Committing `GROQ_API_KEY`** to `lwoodz.toml` or anywhere else. It is only
  ever an environment variable.
- **Ignoring incompatible dependency warnings** without documenting an exception
  or a deliberate decision.
- **Using `lwoodz --daemon` inside an interactive agent step** when a one-shot
  `lwoodz-cli audit` is sufficient.
- **Forgetting to regenerate docs** after changing `[project]` config or
  dependencies.

## Safety

- `lwoodz`/`lwoodz-cli` only write files in the project tree when configured to
  do so (`mode = "enforce"` or explicit `--generate`/`headers`).
- Default `mode` is `"observe"` (report only).
- Always dry-run header changes before applying them.
- The daemon handles `SIGTERM`/`SIGINT` gracefully and finishes in-progress
  checks before exiting.
