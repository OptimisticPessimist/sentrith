# Sentrith Versioning

Sentrith uses SemVer, starting in the **0.x** series.

`0.x` states plainly that the contract is still moving: under SemVer, a major
version of zero means anything may change. That is the accurate description of
Sentrith today, and it is more useful to adopters than a `1.0.0` that promises
stability the project has not yet earned.

`1.0.0` is reserved for the point where the repository contract and the usage
CSV schema are considered stable.

Version sources of truth:

- Cargo package: `tools/sentrith/Cargo.toml`
- Git tag: `sentrith-vX.Y.Z`
- GitHub Release: `Sentrith vX.Y.Z`

Snapshot numbers used before the Sentrith name are not public versions and are
not treated as version history.

## What each number means

Sentrith ships a repository contract and a local CLI, not a library API, so
compatibility is defined by **what an update requires of the user**.

| Bump | Meaning |
|---|---|
| **MINOR** (`0.1.0` → `0.2.0`) | `install.sh --update` migrates the project safely. New contract files, new memory files, new CLI commands, and automatic data migrations all belong here. While in `0.x`, breaking changes also land in MINOR, as SemVer prescribes. |
| **PATCH** (`0.2.0` → `0.2.1`) | Documentation fixes and bug fixes only. Nothing new to adopt, nothing to migrate. |
| **MAJOR** (`0.x` → `1.0.0`) | The contract and the usage CSV schema are declared stable. After that, MAJOR means an update that `--update` cannot complete on its own and that requires documented manual steps. |

The practical test for a MINOR bump: a user on the previous version can run
`install.sh --update`, read the printed summary, and keep working.

If that is not true, the release notes must say exactly what to do by hand, and
`docs/guide/UPDATING.en.md` (and its `.ja` counterpart) must cover it.

## Release checklist

1. Update `version` in `tools/sentrith/Cargo.toml`.
2. Confirm `cargo test` and `cargo build --release` pass on all CI platforms.
3. Note user-facing migrations in `docs/guide/UPDATING.en.md` and `UPDATING.ja.md`.
4. Tag `sentrith-vX.Y.Z`; the release workflow builds and publishes binaries
   plus `SHA256SUMS`.

Keep the CLI and the contract on the same version. An older CLI does not know
newer checks; a newer CLI does not expect older contract paths.
