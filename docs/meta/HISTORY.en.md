<p align="right"><strong>English</strong> ｜ <a href="HISTORY.ja.md">日本語</a></p>

# Sentrith — Design History

`docs/meta/` is human-facing design history and should not be loaded during ordinary feature, bug-fix, refactor, or debugging work.

## Pre-release prototype era

Before the Sentrith name, the project went through several internal design iterations.

Those internal snapshot numbers are **not public Sentrith releases**.

The main evolution was:

1. Split Project Memory into `PROJECT / STATE / DECISIONS / KNOWN_ISSUES`.
2. Add vendor-neutral adapters for coding agents.
3. Introduce Tiny / Normal / Significant development levels.
4. Apply SDD / TDD / ADR only when useful.
5. Add Safety Gates, Evidence Gates, and Human Review.
6. Add a policy against unnecessary extra model calls.
7. Replace runtime-heavy helper scripts with a deterministic Rust CLI.
8. Add usage logging, baseline comparison, and README publication.
9. Add agent-specific automatic usage capture where documented.
10. Generalize measurement to Provider Usage + Task Ledger across IDE/Desktop/CLI.
11. Add anonymized Community Benchmark contributions.
12. Add adaptive Engineering Profiles for Web, AI/ML, Data, and Game/Interactive 3D.

## Sentrith 0.x — Start of the public series

Core message:

> **Ask freely. Build reliably.**

Four pillars:

- **Remember**
- **Structure**
- **Guard**
- **Measure**

Public version history starts at **Sentrith 0.x**. The series stays at 0.x until the contract and the usage CSV schema are considered stable.
