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
.agents/            # canonical cross-agent skills
.claude/skills/     # thin adapters
.claude/settings.hooks.example.json
.codex/hooks.example.json
docs/ai/
docs/development/
docs/profiles/
docs/specs/
docs/rfcs/
```

It does not modify your application source code.

`.agents/` is required: the adapters in `.claude/skills/` point at it.

> If the target already contains Sentrith-managed paths, the installer stops instead of overwriting them.
> **If Sentrith is already installed and you are moving to a new version, use `--update`.** It replaces contract files and preserves project memory.
> `--force` replaces project memory too; use it only to reinstall from scratch.

Details: [Updating](UPDATING.en.md)


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

A fetch script downloads the binary for the current OS from the newest release into the target repository's `bin/`, with SHA256 verification:

```bash
./scripts/get-sentrith.sh /path/to/your-project
```

Windows PowerShell:

```powershell
./scripts/get-sentrith.ps1 -Target C:\path\to\your-project
```

No Rust toolchain is required.

Maintainers can also build it locally:

```bash
cargo build --release --manifest-path tools/sentrith/Cargo.toml
```

The resulting binary is under Cargo's target directory.

After placing `sentrith` on `PATH`, verify the repository:

```bash
sentrith preflight
sentrith guard
```

To enable measurement hooks, run this instead of editing JSON by hand:

```bash
sentrith hooks install
```

It merges only Sentrith's hooks and preserves the rest of your `.claude/settings.json`.

## First real task

Now use your normal coding agent normally:

```text
Fix the login timeout bug.
```

Sentrith should read the repository contract and choose the appropriate engineering depth without requiring a separate classification agent.

## Measure later, not first

If you want to evaluate usage savings, collect a baseline before changing your workflow.

See [Usage Measurement](../metrics/MEASUREMENT_ARCHITECTURE.en.md).
