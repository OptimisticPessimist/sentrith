<p align="right"><strong>English</strong> ｜ <a href="GITHUB_ACTIONS.ja.md">日本語</a></p>

# GitHub Actions

Sentrith ships workflows for CI, releases, usage publication, and community benchmark validation.

## Principles

- CI should be deterministic whenever possible.
- Hooks and CI checks should not call an LLM just to classify routine work.
- The Rust CLI is the preferred local/CI implementation for deterministic checks.
- A green workflow is evidence, not proof that the design itself is good.
- Do not weaken tests or checks merely to make CI pass.

## Main workflows

### Sentrith CI

Builds and checks the Rust CLI.

Typical responsibilities:

- compile `tools/sentrith`
- run tests when present
- catch syntax/build regressions
- verify the repository remains internally consistent

### Release

Builds precompiled binaries for supported platforms and publishes release artifacts from a Sentrith release tag.

Expected public tag format:

```text
sentrith-v1.0.0
```

The source package version is kept in:

```text
tools/sentrith/Cargo.toml
```

### Usage publication

Aggregates benchmark data and updates only the marked benchmark sections in the English and Japanese READMEs.

It must not publish `.ai-usage/usage.csv`, raw prompts, transcripts, source code, customer names, or repository identifiers.

### Community benchmark

Validates anonymized contribution files and aggregates qualified samples.

Community data is explicitly community-reported and should remain separate from maintainer-controlled benchmarks.

## Windows ARM

Windows ARM runner availability can differ from the primary release matrix. Treat it as an optional/preview release target unless the active GitHub-hosted runner environment is verified.

## Safety

GitHub Actions must never become an automatic authorization path for a destructive change.

A workflow can provide independent evidence, but high-impact operations still follow Sentrith's Safety and Human Review policies.
