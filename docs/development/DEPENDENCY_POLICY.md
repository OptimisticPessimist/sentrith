# Dependency Policy

Adding dependencies creates long-term maintenance and supply-chain cost.

Before adding a new dependency, check:

1. Can existing project dependencies solve it?
2. Can the standard library/platform solve it reasonably?
3. Is the dependency materially simpler or safer than a local implementation?
4. Is it maintained?
5. Is its license compatible with the project?
6. Does it introduce install/build scripts or native binaries?
7. Does it materially expand transitive dependencies?
8. Is the selected version pinned/locked by the project's package manager?
9. Is a major-version upgrade actually required?

Do not replace primary frameworks/databases/package managers as a side effect of a narrow task.

For small functionality, avoid large dependencies.

For security-sensitive packages, prefer established, actively maintained libraries over bespoke cryptographic/security implementations.
