# Existing-Repository Bootstrap

Use this once when introducing this project-memory system to an existing repository.

## Bootstrap prompt

Give the coding agent this instruction:

> Initialize the vendor-neutral AI project memory for this existing repository.
>
> First read `AGENTS.md` and `docs/ai/MEMORY_POLICY.md`, then inspect the current repository without changing application behavior.
>
> Inspect, as relevant:
>
> - repository structure
> - README and current documentation
> - dependency manifests and lockfiles
> - application entry points
> - build configuration
> - test configuration
> - lint/format/type-check configuration
> - CI/CD
> - database/schema/migrations
> - external service integrations
> - environment/configuration mechanism
> - current Git status and diff
>
> Populate `docs/ai/PROJECT.md` using only verified stable facts.
>
> Replace bootstrap placeholders in `docs/ai/STATE.md` with the actual current state.
>
> For `docs/ai/DECISIONS.md`, record only durable decisions that are supported by repository evidence. If rationale is unknown, state that it is unknown rather than inventing it.
>
> For `docs/ai/KNOWN_ISSUES.md`, record only recurring or expensive problems supported by current code, tests, logs, documentation, issue references, or reproducible behavior.
>
> Do not migrate the entire Git history or old chat history into project memory.
> Current code and executable configuration are the primary source of truth.
> Consult Git history only when current state does not explain an important design choice or recurring problem.
>
> Do not modify application code merely to fit preferred conventions.
>
> Run the narrowest safe baseline verification commands available and record only their actual outcomes.
>
> Then run the profile questions below and populate `docs/ai/PROFILE.md`.
>
> Finish with:
>
> 1. project-memory files created or changed
> 2. remaining unknowns
> 3. contradictory or apparently stale documentation
> 4. commands actually executed
> 5. baseline verification results
> 6. items that need human confirmation

## Profile questions

Ask these once, during bootstrap, after inspecting the repository.

Propose an answer for each from repository evidence, and ask the user only to
confirm or correct it. Do not ask questions the repository already answers.

Keep it to these; do not invent extra process questions.

1. Which of these does this repository contain? (multiple allowed)
   - externally reachable API / endpoints / schemas
   - database schema or migrations
   - authentication / authorization
   - model or prompt driven behavior (LLM, ML inference, RAG)
   - training data, datasets, or data pipelines
   - real-time / rendering / game-engine content
2. What is the blast radius of a bad change?
   - internal only / affects users / irreversible (money, deletion, public contract)
3. Are there irreversible operations an agent could reach?
4. Which external contracts must not break silently?
5. What verification exists today beyond unit tests?
   (integration, contract, eval, data quality, visual, platform)

Map answers to profiles in `docs/profiles/`:

```text
API / schema / auth / jobs        -> Web / Backend
prompts / models / RAG / eval     -> AI / ML
datasets / pipelines / notebooks  -> Data
engine / scene / asset / platform -> Game / Interactive 3D  (VRChat as subprofile)
```

Enable **every** profile that matches; profiles are additive overlays, not
exclusive modes.

Write the result to `docs/ai/PROFILE.md`:

- enabled/rejected profiles, with the real paths or change kinds that trigger each
- failure impact
- only the techniques actually in force, each with the condition that activated it
- domain verification commands that are not already in `PROJECT.md`

Do not copy technique explanations into `PROFILE.md`. It is an index, and it is
loaded on ordinary tasks, so it must stay small.

Enable a technique only when its gate condition in the profile document is met.
When unsure, leave it disabled and record why.

## Human review after bootstrap

Review the generated project memory once before treating it as canonical.

Pay special attention to:

- inferred architecture
- build/test commands
- statements about security or persistence
- reconstructed historical rationale
- statements copied from stale README files

The main risk during bootstrap is not missing information.
It is confidently preserving incorrect information.

## Periodic cleanup prompt

Occasionally run:

> Audit `docs/ai/` against the current repository.
> Remove stale state, correct contradicted facts, merge duplicate knowledge,
> and retain only information that reduces future rediscovery cost.
> Do not change application behavior during this audit.
