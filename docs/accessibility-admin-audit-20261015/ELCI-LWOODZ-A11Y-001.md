# ELCI-LWOODZ-A11Y-001 — Detect disability-related access barriers

Version 1.0 · Issued 15 October 2026 · Technical implementation directive

Target: lwoodz accessibility assessment and browser-test integrations. These detectors are specified here; this audit does not implement them.

## Purpose and evidence

Detect barriers to understanding, navigating, perceiving and operating a service. Report the affected task, user need, evidence and technical criterion. A WCAG failure is evidence of an accessibility defect; it does not establish every element of unlawful discrimination.

Inputs: source snapshot/file hashes; rendered pages; route/task inventory; browser and accessibility-tree observations; fixture identities; media samples; service/audience/jurisdiction declarations; human assistive-technology results. Dirty source requires file hashes alongside its base commit. A live observation requires URL/time; local source must not be described as the deployed revision without a build receipt.

Outputs: versioned `AccessibilityAssessment/v1`, JSON/SARIF findings, replay fixtures and legal-applicability questions. Each rule declares framework support, static-analysis limitations, browser versions and fixture coverage.

## Legal applicability

For Great Britain, assess reasonable adjustments under Equality Act 2010 sections [20](https://www.legislation.gov.uk/ukpga/2010/15/section/20) and [29](https://www.legislation.gov.uk/ukpga/2010/15/section/29). Section 20 addresses substantial disadvantage and reasonable steps, including accessible information; section 29 applies the duty to service provision. Establish whether this is a public service, workplace tool or another relationship before selecting provisions. Do not extend this mapping automatically to Northern Ireland.

For the US, [DOJ web guidance](https://www.ada.gov/resources/web-guidance/) describes responsibilities of covered businesses and state/local government. Record ADA title, entity coverage, jurisdiction and service nexus. Do not apply a public-sector rule/deadline to every private SaaS service. DOJ guidance and WCAG are not court determinations.

For EU-facing services, assess service categories, national implementation and exemptions under [Directive (EU) 2019/882](https://eur-lex.europa.eu/eli/dir/2019/882/oj). B2B-only use, consumer e-commerce and microenterprise facts matter. EUR-Lex full-text retrieval was bot-blocked in this audit: EU applicability remains unverified.

Emit `potential_reasonable_adjustment_failure` with barrier evidence and applicability questions. Never emit `confirmed_unlawful_discrimination` from an axe score, missing attribute or model opinion. Reviewed legal findings require law version, relevant facts, reviewer and reasoning. Unresolved legal classification must not lower engineering priority for a demonstrated barrier.

## Initial detector classes

Every detector needs a failing fixture, accessible counterexample and explicit exceptions. Additional classes below are future coverage requirements, not additional observed defects; observed findings live in `findings.json`.

| ID | User need / class | Detection and acceptance | Technical mapping and limits |
|---|---|---|---|
| LW-A11Y-001 | Distinguishable page purpose | Direct-load and SPA-transition each task; compare title, purpose and heading. Detect generic/misleading titles on distinct tasks. | WCAG 2.4.2 A. Accept meaningful shared titles for genuinely shared purposes. |
| LW-A11Y-002 | Preserve orientation after navigation | Keyboard-activate route links; capture active element, next sequential focus, scroll and accessible context. Detect demonstrated lost place, restart or obscured focus. | Candidate 2.4.3 A / 2.4.11 AA. BODY focus alone is insufficient; focusing every H1 is not mandatory. |
| LW-A11Y-003 | Bypass repeated blocks | Activate skip mechanism; inspect next Tab/Shift+Tab position and bypassed content in each browser. | 2.4.1 A. A native fragment may correctly set sequential focus even with BODY activeElement; absence of tabindex alone is not failure. |
| LW-A11Y-004 | Hear selected search result | Resolve input/listbox/option relationships. Drive arrows, filtering, empty results and Enter; compare visual and accessible active options and actual destination. | 1.3.1 / 4.1.2 A; AT verification required. Accept coherent native links/buttons instead of a composite pattern. |
| LW-A11Y-005 | Discover asynchronous results | Fixture-test loading, success, failure and reply completion while focus stays at the initiating control. Inspect relevant status semantics and AT announcements without token-by-token speech flooding. | Candidate 4.1.3 AA. Not every DOM update is a status message; an unrelated live region does not qualify replies. |
| LW-A11Y-006 | Operate drawers/dialogs | Record entry focus, background interaction, Tab, Escape, close and restoration at mobile/desktop widths. Determine whether the intended surface is modal or non-modal. | 2.4.3 / 4.1.2 A, 2.4.11 AA as evidenced. Missing trap is not itself a 2.1.2 keyboard-trap failure. Accept accessible non-modal panels. |
| LW-A11Y-007 | Understand grouped panels | Verify tab selection, relationships, panel identity and keyboard behavior, or ordinary toggle semantics where appropriate. Exercise disabled/empty panels. | 1.3.1 / 4.1.2 A. APG arrow-key expectations are diagnostics, not automatic standalone legal violations. |
| LW-A11Y-008 | Review consequential actions | Open remove/revoke/approve; observe prompt discovery and trigger replacement; cancel, confirm and recover from denial. Preserve meaningful focus and the action subject/consequence. | Candidate 2.4.3 / 3.3.2 A; 3.3.4 AA only where its legal/financial/data scope applies. Inline confirmation can be accessible. |
| LW-A11Y-009 | Read at one's own pace | Advance every independent timer with a fake clock. Verify pause/stop/hide/frequency control for automatically updating information, and documented essential exceptions. | 2.2.2 A. Global video pause/reduced-motion preferences cannot qualify a separate image-playlist timer. |
| LW-A11Y-010 | Equivalent media information | Inspect representative content, embedded/external captions, transcripts and descriptions; check language and timing. Separate unsupported external association from actual missing equivalents. | 1.2.1/1.2.2/1.2.3 A, 1.2.5 AA as applicable. No track element alone does not prove absent embedded captions. |
| LW-A11Y-011 | Scale, perceive and reach controls | Test 320 CSS px, 200% text resize, text-spacing overrides, both themes, focus visibility/obstruction, contrast and target spacing. Capture clipping, screenshots and bounds. | 1.4.3/4/10/11/12, 2.4.7/11, 2.5.8 with exceptions. Viewport emulation is not browser zoom certification. |
| LW-A11Y-012 | Complete forms and authentication | Exercise empty/invalid/expired/rejected/busy/success states; labels, error association/discovery, retained input, paste/password managers and alternatives. Inject responses in local fixtures. | 1.3.1/3.3.1/3.3.2/4.1.2 A; 3.3.3/7/8 AA where applicable. Native validation may satisfy a case. |
| LW-A11Y-013 | Understand language and instructions | Compare document/passages and declared language; test locale transitions. Record untranslated instructions separately as usability gaps. | 3.1.1 A / 3.1.2 AA only with actual language mismatch. English-only UI is not automatically disability discrimination. |

## Finding and integration contract

Each finding includes ID/class, affected user need/task, route/component, environment/time, source/build identity, method, reproduction, expected/actual behavior, severity rationale, criterion/level, status (`runtime_confirmed`, `source_confirmed`, `candidate`, `untested`, `rejected`), confidence, exceptions, artifact hashes, remediation acceptance and legal facts still needed.

Use lwoodz's existing directive fields (`id`, `target`, `inputs`, `implementation`, `acceptance_tests`, `evidence_artifact`) when integrating with use-case assessments. Version accessibility output independently and preserve licensing/age-gate contracts.

Inference may suggest semantic mismatches, task scenarios and exception questions from redacted evidence. It cannot set runtime pass, conformance or legal breach. Require exact evidence IDs/source spans; reject invented citations. Pin provider/model/prompt hash; explicitly set temperature 0 when supported and record unsupported controls. Model outputs remain probabilistic and outside trusted policy/test execution. Audit implemented inference calls with TempCheq.

## Acceptance

1. Every rule has failing and accessible fixtures; heuristics produce candidates until their runtime predicates are observed.
2. Native skip behavior passes when next sequential focus correctly bypasses navigation, even with BODY activeElement.
3. A wrong title is reproduced on direct load and SPA transition; a meaningful shared title is accepted.
4. Search selection and actual activation agree in visual state and accessible relationships.
5. Independent timers/results are tested independently; a global pause/live region does not satisfy unrelated components.
6. Missing roles, protected routes, absent media, timeouts and unavailable AT remain explicit coverage gaps, never passes.
7. Human validation covers critical tasks with actual NVDA/browser and VoiceOver/Safari combinations, keyboard, speech input, zoom and representative disabled users. Record each tested combination; emulation is not an AT session.
8. Adjudicated false positives gain regression fixtures. Retain source, browser, accessibility-tree, screenshot and test receipts, then validate artifacts through Deliver.

Prioritize blocked tasks and concealed focus, then names/relationships, result discovery and reading time. User needs remain visible even where a criterion or statutory breach is not established.
