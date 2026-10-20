# Use-case analysis — declared-context risk triage

`lwoodz --analyze <PROFILE>` / `lwoodz-cli analyze --profile <PROFILE>` evaluate a
declared use-case profile (TOML or JSON, see `examples/use-case.toml`) and emit a
`lwoodz.use-case-analysis/v1` report. Everything is **declared context**: the profile
states facts about the target, and the engine matches those facts against dated,
maintained reference tables. No repository scan, control verification or legal
determination is performed, and the output is not legal advice or a compliance
certificate.

## Profile format

Each `[[targets]]` entry declares:

| Field | Meaning |
| --- | --- |
| `target`, `use_case` | Free-text identity and description (required, distinct). |
| `markets`, `activities` | At least one each. Activities: `operate`, `host`, `share_source`, `distribute_binary`. |
| `user_jurisdictions` / `developer_jurisdictions` / `hosting_jurisdictions` | Territory codes. `EU`/`EEA`, `GB`/`UK` alias onto `EU`/`UK`; `US` and `US-CA` are maintained; anything else is an explicit coverage gap. |
| `personal_data`, `sensitive_data`, `children`, `automated_decisions`, `user_content`, `cross_border_transfers` | Tri-state: omitted means **unknown, not false**. |
| `minimum_age` | Lowest age permitted for independent sign-up. Drives consent-band detection. |
| `age_gate` | `self_declared` (user-stated age, unverified) or `verified` (document/ID check, verified parental consent, trusted third-party age signal). |
| `dependency_licenses` | Selected SPDX expressions; a declaration, not a scan. |

`sensitive_data = true` contradicting `personal_data = false` fails validation, as do
duplicate targets, empty markets/activities and misspelled keys
(`deny_unknown_fields`).

## Age gate detection

The engine compares the declared `children` / `minimum_age` / `age_gate` facts
against the consent bands maintained per user jurisdiction:

| Jurisdiction | Parental authorization below | Wider child-protection coverage | Rule |
| --- | --- | --- | --- |
| EU | 16 (member states set 13–16) | 16 | GDPR Art. 8 |
| UK | 13 | 18 | UK GDPR Art. 8; ICO Children's Code |
| US | 13 | 13 | COPPA (directed to under-13s or actual knowledge) |
| US-CA | 16 | 16 | CCPA/CPRA minor opt-in for sale/sharing |

Findings (each with priority, message, remedy and jurisdictions):

- `AGE_SCOPE_UNKNOWN` — neither `children` nor `minimum_age` declared; band checks skipped.
- `AGE_GATE_MISSING` — children declared with no `age_gate` and no `minimum_age`.
- `AGE_GATE_WITHOUT_FLOOR` — a gate is declared without a `minimum_age` floor, so band adequacy cannot be checked.
- `BELOW_AUTHORIZATION_BAND` — declared `minimum_age` sits below a jurisdiction's authorization age. P0 when `children = true`, otherwise P1.
- `CHILD_PROTECTION_CODE_SCOPE` — service falls under wider design-code coverage (e.g. UK under-18s).
- `SELF_DECLARED_GATE_INSUFFICIENT` — self-declared age only while verifiable authorization or sensitive-data duties may apply.

`age_gate.blocked` is true while any P0 finding is open, and an `AGE_GATE`
recommendation is added to the rule engine until the findings are cleared.

## Requirement detection and the jurisdiction indices

`src/jurisdiction.rs` maintains two global indices plus the consent bands above
(schema `lwoodz.jurisdiction-index/v1`, one reviewed date for the whole table):

- **Required measures** — obligations keyed by jurisdiction: EU/UK transfer
  safeguards, EU child authorization, EU sensitive-data/DPIA conditions, EU
  automated-decision safeguards, EU DSA notice-and-action, UK restricted-transfer
  route, UK child consent, UK Children's Code, US COPPA verifiable parental
  consent, California consumer rights, California minor opt-in.
- **Certified methods of implementation** — instruments an authority has
  approved, recognized or codified for a measure: EU SCCs (Implementing Decision
  (EU) 2021/914), EU binding corporate rules, the EU-US Data Privacy Framework
  (Implementing Decision (EU) 2023/1795), the UK IDTA and UK Addendum, UK
  adequacy regulations, the Children's Code standards, FTC-approved COPPA safe
  harbor programs (16 CFR § 312.11(c)), and the Global Privacy Control signal
  (CCPA regulations § 7025). Some measures (EU/UK child consent) have no
  certified method on record — an empty list means "none maintained", not
  "optional".

Requirement detection evaluates every measure trigger against the declared facts
(omitted facts never trigger). Each applicable measure in the report carries its
obligation, legal basis, the triggering facts, and its certified methods.
`lwoodz --jurisdictions` / `lwoodz-cli jurisdictions` dump the full indices on
their own, human-readable or `--json`.

## Epistemic contract

- Dated offline reference set; recheck current law, commencement dates and
  contractual duties before release. Sources are cited per measure and listed in
  the report's `sources` table with the rules-reviewed date.
- Unsupported jurisdictions require local review; missing rules are never an
  inferred pass.
- Detection evaluates declarations; it does not verify a deployed gate, a signed
  instrument, or the current legal text.
