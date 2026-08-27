# Security policy

Report suspected vulnerabilities privately to the repository maintainers. Do
not open a public issue until a fix or coordinated disclosure is available.

Contact: **svch@seriousaboutsolutions.co.uk**

Supported code is the latest `master` branch. `cargo audit` and `cargo deny
check advisories sources` run on every push and pull request via CI.

## Current transitive maintenance watch

None open. `cargo audit` reports 0 known advisories across the current
238-crate dependency tree (last checked 2026-08-15).

## Handling of the Groq API key

`GROQ_API_KEY` is read from the environment only (`src/inference/groq.rs`),
never written to `lwoodz.toml`, never logged, and requests carry a 10-second
timeout. Inference is fully optional — core functionality (detection,
compatibility checks, header management) works offline without a key.
