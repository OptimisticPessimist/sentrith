# Engineering Profile

Status: initialized 2026-09-25

## Enabled profiles

| Profile | Enabled | Trigger |
|---|---|---|
| AI / ML | yes | Decision Engine, model prompt/protocol, provider ranking, routing quality or evaluation changes under `tools/sentrith/src/` |
| Data | no | No data pipeline or analysis dataset is part of the current product |
| Web / Backend | no | No web service in the current CLI |
| Game / Interactive 3D | no | No game/3D code |

AI / ML verification adds a fixed representative routing set, provenance (model ID, prompt/protocol, params), baseline comparison, and failure cases when model-driven quality can change. The current rule-only `route` uses deterministic policy tests. Golden model evaluation activates with a real Decision Engine.

## Failure impact

- Wrong route may consume subscription capacity or choose an inappropriate agent; future Auto execution could affect workspace files.
- Current route does not execute an agent but writes local SQLite metadata. Existing hook/settings writes have local security implications.
- Existing CLI flags and usage records are external contracts.

## Domain verification commands

No model evaluation command exists yet. Standard Cargo commands are in `PROJECT.md`.
