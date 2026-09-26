# Sentrith CLI

Local CLI for Sentrith.

The existing guard and usage commands use the Rust standard library. Router
configuration and metadata storage use Serde/TOML and bundled SQLite.

## Commands

```text
sentrith preflight
sentrith closeout-check
sentrith guard
sentrith review-hint
sentrith diff-budget
sentrith route --priority p2 "review this architecture"
sentrith config init
sentrith config check
sentrith db migrate
sentrith status
sentrith explain <task-id>

sentrith usage record ...
sentrith usage report --compare
sentrith usage note "..."
```

The CLI:

- performs no network requests in the current commands, including `route`
- calls no AI model/API yet; `route` identifies its rule-based decision explicitly
- uses local files and `git` only
- stores usage data under `.ai-usage/` by default

`route` is an initial Suggest command that never executes an agent. It checks local CLI presence
and filters named profiles through a deterministic policy before a provisional
rule selection. It stores task metadata, allowed/rejected profiles, and
structured route events in ignored `.sentrith/sentrith.db`; the original task
text is not stored. Authentication, quota signals, and APUS model ranking are
not implemented yet. See [`docs/ROADMAP.md`](../../docs/ROADMAP.md).

`config init` publishes a complete owner-only template and refuses to overwrite
an existing config. It requires hard-link support in `.sentrith/`; unsupported
filesystems return an error without publishing a partial config. Interrupted
initialization may leave a `.config-init-*.tmp` staging file; retrying uses a new
staging name.

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

## Before changing file-replacement or backup/restore logic

Read [`DECISIONS.md`](DECISIONS.md) first. It records durable engineering
decisions about this implementation whose rationale would otherwise be
expensive to rediscover.
