---
name: dependency-gate
description: Evaluate whether adding, replacing, or major-upgrading a dependency is justified, with supply-chain, maintenance, licensing, transitive dependency, and project-fit considerations.
---

Read `docs/development/DEPENDENCY_POLICY.md`.

Before adding/replacing/upgrading a dependency:

- identify why existing dependencies/platform capabilities are insufficient
- choose the smallest appropriate dependency
- check repository conventions and package manager
- consider maintenance, license, native/install scripts, and transitive footprint
- avoid unrelated major upgrades
- update lockfiles using the repository's normal tooling
- run relevant verification

Do not invoke another model solely for dependency evaluation.
