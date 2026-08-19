# Diff Budget

Large diffs reduce review quality for humans and agents.

The budget is a warning system, not an absolute line limit.

## Principles

Prefer:

- smallest coherent change
- behavior change separate from broad cleanup
- migration separate from unrelated refactor
- generated output clearly distinguishable from hand-written code

## Default warning thresholds

Consider a change "large" when one or more apply:

- more than 50 changed files
- more than 1500 changed non-generated lines
- more than 3 major subsystems
- feature work mixed with broad formatting/refactor churn

For large changes:

1. ask whether the work can be split into independently verifiable increments
2. avoid adding unrelated cleanup
3. identify generated/vendor files separately
4. increase verification breadth
5. use REVIEW-RECOMMENDED at minimum

Do not split a change purely to satisfy an arbitrary number if splitting would make migration/behavior less safe.
