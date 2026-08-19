# Verification Policy

Prompt instructions are guidance.
Executable verification is stronger evidence.

## Verification ladder

Use the narrowest relevant level first:

1. focused regression/unit test
2. relevant integration test
3. type check/static analysis
4. lint
5. build
6. end-to-end/system test
7. visual/manual/operational verification when required
8. broader suite when risk justifies it

Do not run every expensive check for every tiny change.

Do not skip a necessary high-value check to save model credits.

## Changes that normally require broader verification

- public API
- persistence/schema
- auth/security
- concurrency
- serialization
- build/deployment
- shared library/core utilities
- broad refactor

## Test integrity

Passing tests are evidence only if the tests still represent intended behavior.

Do not weaken tests merely to accommodate implementation.

## Verification report

Final report should list only commands/checks actually performed.

Never report:

```text
tests should pass
```

as if they passed.
