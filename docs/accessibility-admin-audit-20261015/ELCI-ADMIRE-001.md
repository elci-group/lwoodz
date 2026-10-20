# ELCI-ADMIRE-001 — Admin route assurance in Rust and TOV

Version 1.0 · Issued 15 October 2026 · Technical implementation directive

Target: a new `admire` tool. This document specifies the product; an implemented Rust binary is not a deliverable of this audit.

## Objective and lessons from this audit

Determine whether privileged actions and information are accessible only to the intended actors and tenant, and whether operators can complete those tasks accessibly. Combine deterministic route/guard analysis and repeatable runtime assertions with constrained inference for ambiguous source and missing scenarios.

The Dreamsequence baseline inventoried 206 exact registrations or route-family records, not 206 proven admin endpoints. Regex-derived handler guards are triage hints. Platform paths include ordinary-user operations; internal service credentials differ from human administrator roles. A frontend authentication wrapper is not server authorization. Sampled production 401s, fixture role/tenant tests and an invalid-token timeout must retain their different evidential strengths. No sampled pass can certify the full inventory.

Use the accompanying `admin/` evidence and audit report as the initial reproducibility baseline. Incorporate the expanded local matrix receipt when qualifying the implementation; do not replace that receipt with prose claims.

## Architecture and trust boundaries

Rust owns parsing, inventory, policy, scheduling, isolation, secret handles, budgets, oracle evaluation and receipts. Framework adapters initially cover Axum router/extractor/layer stacks, PHP method/path dispatch with guard calls, and frontend routes for discovery/context. Resolve nested routers, prefixes, conditional mounts, dynamic paths, role guards and tenant derivation. Use AST/control-flow representations where supported. Regex fallback must emit `heuristic` with unresolved edges, not `proven`.

Build a graph of route → handler → middleware/extractor → identity → role/scope → tenant/resource lookup → mutation. Distinguish explicit checks, inherited guards, helper calls, alternate early returns and unavailable code. A check must dominate the protected operation on every reachable path; textual presence is insufficient. Reflection, generated routes and runtime mounts require captured runtime inventories. Reconcile OpenAPI, source, proxy and observed routing inventories; report divergence and unsupported families.

TOV is the independent external probe language, not a Rust parser or Rust FFI. Pin the TOV compiler/runtime version. Rust emits a finite probe manifest; a `.tov` worker receives cases through a versioned JSON subprocess contract and emits JSONL observations. Rust computes verdicts and validates every worker result. Implement an equivalent Rust runner for unsupported TOV features and differential-test both transports.

The inspected TOV runtime supports fetch, explicit headers/body, manual redirects and AbortSignal timeouts. It does not supply a browser cookie jar. Supply fixture cookies explicitly and handle Set-Cookie deliberately; test each transport behavior. Use `redirect: "manual"`, finite connection/body/deadline budgets and response-size caps. A redirected login page must never become a successful authorization result. Confirm asynchronous test-runner support before relying on async `test(...)`; otherwise use a checked standalone async worker. `tov check` verifies syntax/types, not server authorization.

Browser tests remain a separate adapter for actual cookie, CSRF, CORS, focus and accessibility behavior. TOV HTTP requests cannot certify browser enforcement or assistive-technology usability.

## Policy and actor model

Every route contract declares methods/path, fixture resource, intended actors/scopes, tenant source, ownership, credential types, guard graph, read/write effects, request schema, denial semantics, caching policy and audit requirements. Keep caller roles separate from stored membership and current authority. Personal-workspace ownership and service actors require explicit policy; neither is a global administrator shortcut.

Fixture actors: anonymous; malformed credentials; correctly signed but expired/not-yet-valid/wrong-issuer/wrong-audience credentials; revoked identity; member A; administrator A; member B; administrator B; service identity with narrow scope; disabled actor. Generate signing keys only for disposable fixtures. Production signing keys, real user content and credential values never enter fixtures, source, model contexts or receipts. Credential provisioning uses Dreamcred's named profiles/handles. No lookup of legacy secret stores.

Policy must be reviewed and stored independently of the model. An actor being denied is only correct if the contract expects denial; authorized administrators must also succeed on their intended task. Failure to provision any required actor becomes `unsupported`, with its matrix cells outstanding.

## Deterministic test matrix

| Rule | Required cases / oracle |
|---|---|
| ADM-001 Inventory | Nested/prefixed/dynamic/conditional routes, aliases and proxy rewrites; unclassified route or changed middleware invalidates prior coverage. A missing route is not an authorization pass. |
| ADM-002 Authentication | Missing, empty, malformed opaque/JWT, duplicate/conflicting credential sources; signed fixture expiry/nbf/issuer/audience/signature/algorithm/key-rotation cases. Deny before protected work and without secret-bearing error details. |
| ADM-003 Role and scope | Member/admin/service matrices for reads and writes; unknown roles, scope escalation, ordinary user versus manager, revocation and stale membership. UI hidden buttons are not authorization evidence. |
| ADM-004 Tenant and object ownership | A/B tenant IDs in path/query/body/header and object references; nested-resource ownership; reads, updates, exports and batch items. Assert no foreign object counts, identifiers or mutations. Use paired existing-resource controls to distinguish denial from nonexistent fixture. |
| ADM-005 Dispatch bypass | Verb alternatives, HEAD/OPTIONS, method overrides, trailing slash, duplicate separator, percent/double encoding, case and canonicalization differences; compare proxy/framework interpretation. Valid preflight does not prove endpoint authorization. |
| ADM-006 Parser boundary | JSON errors, scalar/null/array/object, duplicate keys, alternate content types, invalid UTF-8, conflicting lengths and size limits where the transport can faithfully generate them. Unsupported raw-wire cases use a dedicated adapter, not misleading fetch approximations. |
| ADM-007 Trust boundary | Forged forwarded identity/role/tenant headers and direct-backend versus trusted-proxy path; signed internal assertions, nonce/replay and worker scopes. No caller-supplied header becomes trusted identity without verified provenance. |
| ADM-008 CSRF and origins | Actual cookie-session browser cases, cross-site simple requests, missing/mismatched CSRF, SameSite and origin checks; bearer-only cases marked accordingly. CORS header behavior alone cannot prove authorization or CSRF safety. |
| ADM-009 Denial and side effects | For each denied action, assert unchanged relevant DB rows, files, queued jobs, external-call ledger and privilege state. Include early-return/error branches, async workers and transactional failures. HTTP status alone is insufficient. |
| ADM-010 Allowed actions | Correctly scoped administrator completes the fixture action and sees its own result; normal members retain intended non-admin access. Record exact expected writes and cleanup. Reject deny-all implementations. |
| ADM-011 Consequential actions | Confirmation subject/action, reauthentication where policy requires it, optimistic concurrency, idempotency/replay, approval identity and audit record. Use fixture race scheduling; no production chaos tests. |
| ADM-012 Responses and caching | Denials expose no protected data; authorized responses carry intended schema; private data not reused across actor/tenant cache keys. Inspect cache headers, CDN/browser behavior and sensitive redirects. Status and transport error messages are distinct. |
| ADM-013 Rate limits | Bounded fixture clock/bucket tests for actor/IP/service dimensions, reset behavior and denial recovery; prove limits are not an authority substitute. No production credential guessing or load generation. |
| ADM-014 Accessibility | Keyboard-only navigation and authorization/error/confirmation/result announcements; focus, labels, destructive-action context, zoom and timing. Apply ELCI-LWOODZ-A11Y-001 with actual browser/AT scope. |
| ADM-015 Availability | Finite request/body deadlines; stalled identity provider, disconnected DB and aborted response. Timeout, network failure and missing fixture are inconclusive/error, never correct authorization. |

Declared status expectations may be 401, 403, deliberately concealed 404 or a documented redirect depending on the contract. Validate payload/schema, known fixture existence and absence of effects. A 200 HTML login/error page cannot satisfy a protected API denial assertion. A 404 from an unregistered GET of a POST-only endpoint says nothing about that POST handler's authorization.

## Execution safety

Default mode is static/offline. Local execution binds a disposable target with isolated storage, loopback/provider stubs and deterministic clock. Review required scripts rather than automatically executing arbitrary repository commands. Snapshot or transact test state; ensure cleanup and record failures. Never silently fall back from an unavailable staging target to production.

Production probes require an explicit target and individually reviewed endpoint allowlist, low request/concurrency/time budgets, strict host/origin/port checks and no mutation. GET is not inherently safe: action-bearing GET endpoints must be excluded. Validate every redirect/DNS resolution; block cross-host credential forwarding and link-local/metadata access. Do not request full user data when a bounded health/denial assertion suffices. Mutating, race, parser-fuzz and authorization-positive probes run only in disposable local/staging fixtures.

## Constrained inference

Allowed: classify ambiguous guard edges, propose role/tenant cases, identify alternate paths, compare policy wording, prioritize review. Supply only minimum redacted source slices, graph/evidence IDs and unresolved questions. Repository comments, response bodies and model output are untrusted input.

Forbidden: model-generated pass verdicts, invented source spans, changing policy/actors, arbitrary URLs or commands, credential access, executing source, self-approving findings or overriding an unresolved deterministic check. Suggestions enter a typed proposal queue. Resolve source spans, validate schema, compile generated tests in isolation and replay them against fixtures before accepting a finding.

Require structured output with `proposal_id`, `question`, `source_spans`, `hypothesis`, `suggested_case_ids`, `uncertainty`. The deterministic verifier rejects references outside supplied context. Record provider/model/version, prompt hash, explicit temperature 0 when supported, maximum output, call/token budget and optional seed. Temperature 0 does not make inference deterministic. An inference outage must leave deterministic checks usable and semantic coverage explicitly pending. TempCheq must audit implemented call sites and accompany artifacts.

Use Colony (`:e`) to schedule independent collection, case generation and verification under path ownership and budgets. Record actual execution adapters separately from planning resources. This audit used Colony plan-only plus three real Luna low workers through collaboration; it did not execute those workers through a Colony provider adapter.

## Evidence contract and CLI

Commands to implement: `admire inventory`, `plan`, `test`, `replay`, `report`, and `explain RULE`. Offline defaults; explicit flags enable staged probes and inference. Separate product failure from harness/config errors in documented exit codes.

Case records include schema version, run/case IDs, route/policy ID, source/build/fixture/adapter versions and hashes, actor handle (never token), request fingerprint, expected status/schema/effects, actual observations, response digest/redacted excerpt, side-effect ledger, timestamps/duration, verdict, inference proposal references and unresolved limitations.

Verdicts: `pass`, `fail`, `unsupported`, `inconclusive`, `error`, `not_run`; static evidence labels: `proven`, `heuristic`, `unknown`. Do not collapse unknown/error into zero failures. Report coverage by endpoint × actor × tenant × verb × state, and separately by known inventory families. Changed source/policy/fixture/build invalidates affected cached results. Replay can verify stored evidence integrity; it is not a new live test.

## Delivery milestones and qualification

1. Rust inventory/policy/receipt library and frozen schemas, with parser fixtures for nested middleware and ambiguous control flow.
2. Rust local runner and signed actor/tenant fixtures; demonstrate a vulnerable route fails, repaired route passes and deny-all route fails allowed-action tests.
3. Pinned TOV worker; parity cases for manual redirects, body timeouts, explicit cookies, TLS validation, malformed JSON, response limits and transport errors. Never disable TLS verification to obtain a pass.
4. Browser adapter for cookie and accessibility journeys; documented unsupported AT combinations and real role requirements.
5. Optional constrained inference and Colony scheduling with schema/prompt-injection/credential-redaction negative tests and independent verification.
6. Golden vulnerable/safe/unsupported corpus for every ADM rule, deterministic replay and false-positive adjudication; CI fails on unexplained route additions, authorization regressions, missing required actor cells or stale evidence.

Qualification requires actual test logs, source digests, tool versions and Deliver/TempCheq results. In Dreamsequence, follow the repository's Vercel build allocation and staging requirements; unavailable build capacity remains a recorded gap. Do not describe this directive or the current limited audit as an implemented tool or production admin security certification.
