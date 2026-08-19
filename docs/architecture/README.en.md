<p align="right"><strong>English</strong> ｜ <a href="README.ja.md">日本語</a></p>

# Architecture Documentation

This directory is for **the current architecture of the product being built with Sentrith**.

It is not the place for Sentrith's own design philosophy or template history. Those belong in `docs/meta/`.

## Put here

- current system boundaries
- component responsibilities
- runtime topology
- data flows
- external-service boundaries
- deployment architecture
- important cross-cutting technical constraints

## Do not put here

- old alternatives that are no longer relevant
- Sentrith template design history
- long chat transcripts
- generic engineering advice
- speculative architecture that has not been adopted

For durable decisions and rationale, use `docs/ai/DECISIONS.md`.

## Why this separation exists

Architecture documents are normal development evidence. Agents may need them during feature work, debugging, or review.

By contrast, `docs/meta/` should stay outside ordinary task context so template philosophy does not pollute repository search or consume context budget.
