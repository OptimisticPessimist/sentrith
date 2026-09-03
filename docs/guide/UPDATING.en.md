<p align="right"><strong>English</strong> ｜ <a href="UPDATING.ja.md">日本語</a></p>

# Updating Sentrith in an existing project

After extracting a newer Sentrith release or pulling the repository, run this from the Sentrith directory:

**macOS / Linux**

```bash
./scripts/install.sh --update /path/to/your-project
```

**Windows PowerShell**

```powershell
./scripts/install.ps1 -Target C:\path\to\your-project -Update
```

If you use the CLI, move it to the same version:

```bash
./scripts/get-sentrith.sh /path/to/your-project
```

---

## `--update` vs `--force`

**Use `--update`.**

| | `--update` | `--force` |
|---|---|---|
| Contract files (AGENTS.md, skills, policies, profiles) | replaced | replaced |
| Project memory (PROJECT.md, STATE.md, PROFILE.md, DECISIONS.md, KNOWN_ISSUES.md) | **preserved** | **overwritten** |
| Your feature specs (`docs/specs/<feature>/`) | preserved | preserved |
| `.claude/settings.json`, `.codex/hooks.json` | preserved | preserved |

`--force` exists to reinstall from scratch. Using it to update resets accumulated project memory to empty templates.

A plain install with no flags **stops** when it detects existing files. That is deliberate.

---

## What an update does

1. Replaces contract files with the new version
2. Adds any **memory file introduced by that version** as an uninitialized template, without touching existing ones
3. Prints what it added

Example:

```text
New memory files added as uninitialized templates:
  docs/ai/PROFILE.md
```

---

## After updating

### 1. Read the diff

```bash
git diff -- AGENTS.md docs/development docs/profiles .agents
```

This assumes the contract files are committed in the target repository. If you cannot read a diff, they probably are not.

### 2. Fill in newly added memory files

Ask your coding agent:

```text
Run the profile questions in docs/ai/BOOTSTRAP.md and populate docs/ai/PROFILE.md.
Do not change application behavior.
```

Leaving it uninitialized is safe; that feature is simply inactive.

### 3. Re-apply local modifications

If you edited files such as `AGENTS.md`, those edits were replaced. Check `git diff` and re-apply what you need.

**Avoid editing Sentrith-owned files directly.** Put project-specific rules where an update cannot remove them:

- record conventions in `docs/ai/PROJECT.md` rather than appending to `AGENTS.md`
- keep local context you do not want to share in `.sentrith-private/`

### 4. Run the deterministic checks

```bash
sentrith preflight
```

---

## Changes that need attention

### Usage CSV schema

When the `.ai-usage/usage.csv` schema changes, `sentrith` migrates it **in place on the next write**.

- existing rows are preserved and new columns are filled empty
- migration is idempotent
- a benchmark in progress can continue, because aggregation resolves columns by name

Copy the file first if you want to keep the pre-migration version.

### Hook configuration

`.claude/settings.hooks.example.json` and `.codex/hooks.example.json` are **examples** and are replaced.

The files you actually use — `.claude/settings.json` and `.codex/hooks.json` — are not.

So **hook changes do not apply automatically.** Diff the examples and merge what you need:

```bash
git diff -- .claude/settings.hooks.example.json .codex/hooks.example.json
```

### CLI and contract versions

The CLI and the contract can be updated separately, but **keep them on the same version**. An older CLI does not know newer checks, and a newer CLI does not expect older contract paths.

```bash
sentrith version
```

---

## Safe vs risky

Safe:

- replacing contract files (when they are tracked in Git)
- adding new memory files

Risky:

- updating with `--force` (project memory is lost)
- editing Sentrith-owned files directly (lost on every update)
- not committing Sentrith files in the target repository (no diff to review)
