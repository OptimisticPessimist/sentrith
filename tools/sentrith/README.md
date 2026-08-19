# Sentrith CLI

Zero-runtime-dependency local CLI for Sentrith.

The Rust source uses only the standard library.

## Commands

```text
sentrith preflight
sentrith closeout-check
sentrith guard
sentrith review-hint
sentrith diff-budget

sentrith usage record ...
sentrith usage report --compare
sentrith usage note "..."
```

The CLI:

- performs no network requests
- calls no AI model/API
- uses local files and `git` only
- stores usage data under `.ai-usage/` by default

## Build locally

Only maintainers need Rust:

```bash
cargo build --release --manifest-path tools/sentrith/Cargo.toml
```

End users should use a prebuilt GitHub Release binary.

## Tests

```bash
cargo test --manifest-path tools/sentrith/Cargo.toml
```
