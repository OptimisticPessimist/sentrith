<p align="right"><strong>English</strong> ｜ <a href="CONTRIBUTING.ja.md">日本語</a></p>

# Contributing to Sentrith

Thanks for helping improve Sentrith.

You do **not** need to contribute code to contribute meaningfully. Sentrith accepts improvements in three broad areas:

1. **Code & automation** — Rust CLI, hooks, CI, provider adapters.
2. **Documentation & engineering profiles** — guides, examples, domain profiles, translations.
3. **Measurement & benchmark data** — anonymized qualified benchmark contributions.

## 1. Before you start

Read these first when your change affects behavior:

- [Development Method](docs/development/DEVELOPMENT_METHOD.md)
- [Verification Policy](docs/development/VERIFICATION_POLICY.md)
- [Safety Gates](docs/development/SAFETY_GATES.md)
- [Dependency Policy](docs/development/DEPENDENCY_POLICY.md)

For small documentation fixes, you usually do **not** need to read the full policy set.

## 2. Development setup

### Requirements

- Git
- Rust toolchain only if you modify `tools/sentrith`
- No Python runtime is required by Sentrith users

Build the CLI:

```bash
cargo build --manifest-path tools/sentrith/Cargo.toml
```

Run tests:

```bash
cargo test --manifest-path tools/sentrith/Cargo.toml
```

Run deterministic checks from the repository root when the binary is available:

```bash
sentrith preflight
sentrith guard
sentrith closeout-check
```

> The packaged source may be inspected without Rust, but changes to the Rust CLI should be compiled and tested before merge.

## 3. Pick the smallest contribution path

### Code contribution

Good candidates:

- deterministic CLI checks
- provider usage adapters based on documented interfaces
- CI/release improvements
- portability fixes
- bug fixes with reproducible evidence

Avoid introducing a new runtime dependency or model call when deterministic code can solve the same problem.

### Documentation / profile contribution

Good candidates:

- clearer examples
- new domain-specific verification guidance
- corrections to stale docs
- English/Japanese parity improvements
- engineering-profile questions that reduce methodology jargon

User-facing docs that exist in both languages should be updated in both languages in the same PR when practical.

### Benchmark contribution

You can contribute anonymized aggregate data without source code or prompts:

```bash
sentrith usage contribute --agent <agent> --model "<model>"
```

See [Community Benchmark](docs/metrics/COMMUNITY_BENCHMARK.en.md).

## 4. Before changing behavior

Keep the scope explicit.

For normal work, record the intended outcome and acceptance criteria in the PR description or relevant task/spec.

Escalate to a fuller SPEC/PLAN only when the change is significant, such as:

- public compatibility changes
- database/data migration behavior
- authentication or authorization
- security-sensitive behavior
- major provider integration
- architecture changes
- multi-subsystem changes

Do not create process artifacts just because Sentrith supports them.

## 5. Verification

Use evidence appropriate to the change.

Examples:

- Rust code → compile + tests + focused behavior check
- docs → link/path review + language parity where applicable
- benchmark code → fixture/input validation + privacy-field checks
- provider adapter → documented provider interface + failure/fallback behavior
- Game/3D profile → runtime/visual/platform verification guidance where relevant

A green test suite is evidence, not automatic proof that the design is correct.

## 6. Pull request checklist

Before opening a PR, check:

- [ ] Scope is narrow and described clearly.
- [ ] Tests/checks were added or updated when meaningful.
- [ ] Existing tests/checks pass for the affected area.
- [ ] Safety Gates are satisfied for high-impact changes.
- [ ] No test or guard was weakened just to make CI green.
- [ ] English/Japanese user-facing docs were kept in sync when applicable.
- [ ] Project Memory was updated only if durable repository knowledge changed.
- [ ] Benchmark/privacy rules are preserved if measurement code changed.
- [ ] Breaking changes are either avoided or explicitly documented and reviewed.

## 7. Security-sensitive changes

Do not disclose a real vulnerability in a public issue before a private reporting path exists for the repository hosting Sentrith.

For security-sensitive code changes:

- minimize the diff
- preserve independent evidence
- do not weaken auth, secrets, crypto, or checks without explicit justification
- follow `REVIEW-REQUIRED` when the policy requires it

## 8. Language policy

Canonical landing page:

```text
README.md      English
README.ja.md   日本語
```

For paired detailed docs:

```text
*.en.md
*.ja.md
```

See [Language Policy](docs/meta/LANGUAGE_POLICY.md).

## 9. Community behavior

Be specific, evidence-based, and respectful.

Disagreement about architecture, methodology, benchmarks, or product direction is welcome. Personal attacks and gatekeeping are not.

## 10. What makes a strong Sentrith contribution?

A strong contribution usually does at least one of these:

- reduces repeated rediscovery
- improves verification quality
- makes a safety boundary more executable
- lowers unnecessary AI usage without lowering correctness
- makes the system easier to adopt
- makes a domain profile more accurate
- improves measured evidence rather than marketing claims

> **Measured, not promised.**
