# Usage Measurement v2 Implementation Plan

Specification: `SPEC.md`
Last updated: 2026-08-22

## Approach

Extend the existing single-file Rust CLI in place. All new parsing is line-oriented over provider transcripts, reusing the existing `json_string_field` / `json_number_field` helpers plus new targeted extractors (`msg_` / `toolu_` id scoping). No new dependencies.

## Affected areas

| Area/path | Intended change |
|---|---|
| `tools/sentrith/src/main.rs` | schema v2 + migration, transcript window parser, verification detection, HEAD tracking, phase env resolution, `report --tasks` / `--churn`, tests |
| `.claude/settings.hooks.example.json` | drop hardcoded `--phase standard` |
| `.codex/hooks.example.json` | drop hardcoded `--phase standard` |
| `docs/metrics/MEASUREMENT_ARCHITECTURE.*` | v2 semantics: success definition, unknown, churn, `SENTRITH_PHASE` |
| `docs/metrics/BENCHMARK_GUIDE.*` | baseline via `SENTRITH_PHASE` |
| `docs/automation/SENTRITH_CLI.*` | new flags and report modes |

## Data / API changes

CSV schema v2 appends `head_sha,verification` before `notes`? No — appended at the end after `notes` to keep v1 column order untouched. In-place migration rewrites the header and pads old rows with empty columns. Readers are header-indexed, so mixed tooling reading v2 files by name keeps working.

## Verification strategy

| Acceptance criterion | Verification |
|---|---|
| AC-001..AC-005 | Rust unit tests with string fixtures and temp dirs |
| AC-006 | `sentrith-ci.yml` matrix run on push |

Local `cargo` is unavailable on the author machine; CI is the executable gate.

## Risks

- Risk: transcript format drift breaks parsing silently.
  - mitigation: graceful fallback to statusLine cost delta; `source`/`notes` record which path produced the row.
- Risk: schema migration corrupts an existing CSV.
  - mitigation: migration writes to a temp file then renames; unit-tested round trip.
- Risk: test-command regex misses project-specific runners.
  - mitigation: pattern list is a single const; documented; `verification` stays empty rather than guessing.

## Durable decisions

- Objective success proxy definition (candidate for `docs/ai/DECISIONS.md` after adoption).
