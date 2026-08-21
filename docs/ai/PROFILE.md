# Engineering Profile

> Which engineering profiles are enabled for this repository, and when they apply.
> This file is loaded on ordinary tasks. Keep it short (target: <= 100 lines).
> It is an index, not documentation. Technique explanations live in `docs/profiles/`.

Status: not initialized

Initialize this file once by answering the profile questions in `docs/ai/BOOTSTRAP.md`.
Until then, agents apply Sentrith Core only (no domain profile).

## Enabled profiles

| Profile | Enabled | Triggers in this repository |
|---|---|---|
| Web / Backend | unknown | |
| AI / ML | unknown | |
| Data | unknown | |
| Game / Interactive 3D | unknown | |

Replace `unknown` with `yes` or `no`.

Triggers must name real paths or change kinds in this repository, for example:

```text
Web / Backend | yes | src/api/**, migrations/**, anything touching auth
AI / ML       | yes | src/rag/**, prompts/**, eval/**
```

Profiles are additive: a change may match several, in which case apply the union
of their verification dimensions. See `docs/profiles/README.md`.

## Failure impact

- Blast radius of a bad change:
- Irreversible operations present:
- Externally visible contracts present:

## Enabled techniques

Record only techniques that are actually in force, with the condition that
activated them. Do not list techniques "for completeness".

| Technique | Applies to | Why enabled |
|---|---|---|
| | | |

## Domain verification commands

Verification commands beyond the standard ones in `PROJECT.md`
(for example eval runs, contract tests, data quality checks).

```sh
# TODO or "none"
```

## Not applicable

State explicitly which profiles were considered and rejected, so future agents
do not re-litigate the decision.
