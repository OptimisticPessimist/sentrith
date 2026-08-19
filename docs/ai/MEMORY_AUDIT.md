# Project Memory Audit

Use this periodically or when project memory becomes noisy.

The audit must not change application behavior.

## Procedure

1. Compare `docs/ai/` with the current repository.
2. Remove statements contradicted by current source/configuration/tests.
3. Remove resolved transient state from `STATE.md`.
4. Merge duplicate decisions and known issues.
5. Mark superseded decisions explicitly when historical rationale still matters.
6. Remove troubleshooting entries that no longer apply to supported versions.
7. Replace copied source/log material with concise summaries and source references.
8. Check that adapter files do not duplicate canonical project knowledge.
9. Check high-frequency file sizes against `MEMORY_POLICY.md`.
10. Report all material removals or corrections.

## Success criteria

After cleanup:

- `PROJECT.md` is a concise map of stable current truth.
- `STATE.md` can be read quickly and contains no historical diary.
- `DECISIONS.md` answers important "why?" questions.
- `KNOWN_ISSUES.md` accelerates recurring debugging.
- routine tasks do not need to read low-relevance history.
