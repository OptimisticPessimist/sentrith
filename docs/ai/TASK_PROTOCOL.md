# AI Task Protocol

Use this for substantial tasks that benefit from explicit planning.
Small localized edits do not need a formal plan.

## 1. Define the observable outcome

State the requested result in one or two sentences.

## 2. Identify constraints

Check for:

- compatibility requirements
- public API constraints
- performance requirements
- security requirements
- migration constraints
- deployment constraints
- files/subsystems that must not change

## 3. Gather targeted evidence

Read only enough to answer:

- where does the behavior originate?
- what calls it?
- what depends on it?
- how is it tested?
- what existing conventions apply?

## 4. Plan in verifiable increments

Each step should create a state that can be checked.

Example:

1. add regression test reproducing failure
2. fix parser boundary condition
3. update dependent type
4. run parser tests
5. run full type check

## 5. Implement incrementally

After meaningful changes:

- inspect the diff
- run the narrow relevant check
- revise the hypothesis if evidence disagrees

## 6. Verify

Record only commands actually executed and their outcomes.

## 7. Preserve reusable knowledge

Ask:

- Did we discover a stable architecture/workflow fact?
  - update `PROJECT.md`
- Did we make a durable engineering decision?
  - update `DECISIONS.md`
- Did we solve a recurring expensive trap?
  - update `KNOWN_ISSUES.md`
- Did current status materially change?
  - update `STATE.md`

Do not record knowledge merely because something happened.
Record it when a future agent would otherwise pay a meaningful rediscovery cost.
