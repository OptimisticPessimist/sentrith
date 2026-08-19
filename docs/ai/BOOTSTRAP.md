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
> Finish with:
>
> 1. project-memory files created or changed
> 2. remaining unknowns
> 3. contradictory or apparently stale documentation
> 4. commands actually executed
> 5. baseline verification results
> 6. items that need human confirmation

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
