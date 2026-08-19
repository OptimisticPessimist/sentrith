<p align="right"><strong>English</strong> ｜ <a href="INSTALLATION.ja.md">日本語</a></p>

# Installation

Sentrith has two parts:

1. **Repository contract** — files that live in your project and are read by coding agents.
2. **Optional Rust CLI** — deterministic checks and usage measurement.

You can use the repository contract without installing the CLI.

## Option A — install into an existing repository

Download or clone a Sentrith release, then run the helper from the Sentrith directory.

### macOS / Linux

```bash
./scripts/install.sh /path/to/your-project
```

### Windows PowerShell

```powershell
./scripts/install.ps1 -Target C:\path\to\your-project
```

The helper copies the vendor-neutral repository contract, including:

```text
AGENTS.md
CLAUDE.md
.github/copilot-instructions.md
.github/prompts/
.claude/
.codex/
docs/ai/
docs/development/
docs/specs/
docs/rfcs/
```

It does not modify your application source code.
> If the target already contains Sentrith-managed paths, the installer stops instead of overwriting them. Merge intentionally, or use `--force` / `-Force` only when replacement is deliberate.


## Bootstrap Project Memory

From your target repository, ask your coding agent:

```text
Read docs/ai/BOOTSTRAP.md and perform the bootstrap for this repository.
Do not change application behavior.
```

Then review these once:

```text
docs/ai/PROJECT.md
docs/ai/STATE.md
```

The important step is reviewing generated project facts before treating them as durable memory.

## Optional CLI

End users should prefer a prebuilt `sentrith` binary from a release once release assets are available.

Maintainers can build it locally:

```bash
cargo build --release --manifest-path tools/sentrith/Cargo.toml
```

The resulting binary is under Cargo's target directory.

After placing `sentrith` on `PATH`, verify the repository:

```bash
sentrith preflight
sentrith guard
```

## First real task

Now use your normal coding agent normally:

```text
Fix the login timeout bug.
```

Sentrith should read the repository contract and choose the appropriate engineering depth without requiring a separate classification agent.

## Measure later, not first

If you want to evaluate usage savings, collect a baseline before changing your workflow.

See [Usage Measurement](../metrics/MEASUREMENT_ARCHITECTURE.en.md).
