# tools/sentrith — Engineering Decisions

Durable engineering decisions about the Rust CLI implementation in this
directory, whose rationale would otherwise be expensive to rediscover.

This file is **not** shipped with Sentrith: it is not listed in
`scripts/install.sh`'s `CONTRACT_FILE` or `SEED_FILE` manifests, unlike
`docs/ai/`, which ships as a blank starter template to every project
that installs Sentrith and must stay pristine in this repository for
that reason. This file is Sentrith's own development memory about its
own Rust implementation — the place for that memory when it doesn't fit
`docs/meta/` either (which is curated, human-facing product history and
design philosophy, not implementation-level ADRs).

## Entry template

### ADR-YYYYMMDD-NN — Short decision title

**Status:** accepted | superseded | deprecated

**Context**

What problem or constraint required a decision?

**Decision**

What was chosen?

**Rationale**

Why was this chosen for this codebase?

**Alternatives considered**

- Alternative A — why rejected
- Alternative B — why rejected

**Consequences**

- positive consequence
- tradeoff or constraint
- follow-up obligation

**Affected areas**

- `path/to/file`

---

### ADR-20260823-01 — Reuse the hardened file-replace primitives; never a raw fs::write/copy/rename on a security-sensitive path

**Status:** accepted

**Context**

During PR #1 (`feat/usage-measurement-v2-and-profiles`), the hook-settings
backup/restore/rollback logic in `tools/sentrith/src/main.rs`
(`reduce_hook_settings_for_baseline`, `restore_hook_settings_backup`,
`roll_back_committed_replacement`) went through dozens of Codex review
rounds. Several, independently, were the same root-cause class recurring:
a *new* call site wrote or replaced a settings-like file with a plain
`fs::write`, `fs::copy`, or bare `fs::rename` instead of the hardened
primitives that already existed in the same file for exactly this
purpose — and reintroduced a bug that a *different* call site had
already been fixed for rounds earlier. Concretely, this happened at
least three times: a fresh temp file written at default permissions
(exposing restricted settings content), a predictable temp/backup path
followed through an attacker-plantable symlink, and — most recently
(`7f6d0fa`, `48f0655`) — a rollback that called `fs::copy` straight onto
the destination and failed outright once that destination was read-only
(Unix mode `0400`, or the Windows read-only attribute), even though
`restore_hook_settings_backup`'s normal restore path had already solved
that exact problem several rounds before.

Each recurrence cost a full review round to rediscover, because the fix
was locally correct-looking (it compiled, it passed the happy path) and
nothing forced the new code to go through the already-hardened path.

**Decision**

Any code path in this project that replaces a file which may be
permission-restricted, ACL-restricted, or reachable through a
predictable/attacker-plantable path (settings files, ledgers, hook
backups — anything under `.claude/`, `.codex/`, `.sentrith-private/`, or
similar) must use the existing hardened primitives rather than a raw
`fs::write` / `fs::copy` / `fs::rename`:

- `create_secure_file` to create a fresh temp file (refuses to reuse or
  follow anything already at the path — closes the symlink-plant window).
- `replace_file_preserving_security` to atomically swap it into place
  (preserves the destination's permissions/ACL/read-only attribute
  across the swap; works even when the destination is currently
  read-only, since the swap only needs write access to the containing
  directory, not the target file itself).
- `restore_file_content` (added in `48f0655`) for the common "put a
  known-good copy back in place of whatever `destination` currently
  holds" shape — used by both the normal hook-settings restore and
  `roll_back_committed_replacement`'s rollback, so this shape now has
  exactly one implementation instead of two that could drift apart.

Before writing a new file-replacement code path anywhere in
`tools/sentrith/src/main.rs`, grep for these three helpers first and
reuse one of them; do not hand-roll a fourth variant.

**Rationale**

`fs::write`/`fs::copy`/`fs::rename` used directly have well-known,
already-discovered failure modes in this codebase: following a symlink
planted at a predictable path, resetting to default permissions on a
fresh file, losing a Windows ACL/security descriptor, and failing
outright against a read-only destination. Re-deriving these same fixes
independently per call site costs a full review round each time a new
call site skips the existing hardened primitive — this file paid that
cost multiple times for variations of the identical underlying issue.
Reuse is strictly cheaper than rediscovery.

**Alternatives considered**

- Leave call sites bespoke and rely on review to catch regressions each
  time — rejected: this is exactly what happened, repeatedly, at real
  review-cycle cost, and is what this decision exists to stop.
- Add an automated lint/test forbidding raw `fs::write`/`fs::copy`
  against paths under the security-sensitive directories — considered as
  a follow-up; not implemented yet (see Consequences).

**Consequences**

- positive: `roll_back_committed_replacement` and
  `restore_hook_settings_backup`'s restore path now share one
  implementation (`restore_file_content`); a future fix to
  file-replacement semantics lands in one place instead of needing to be
  ported to every call site by hand.
- tradeoff: `restore_file_content`, `replace_file_preserving_security`,
  and `roll_back_committed_replacement` now have a mutual-recursion
  relationship on Windows (verified bounded/non-looping at the time of
  `48f0655` — a rollback's inner swap call cannot itself hit the
  attribute-restore-failure branch that triggers another rollback,
  because the attribute is already non-readonly by the time the inner
  call runs). A future change to any of the three needs to re-verify
  this termination argument, not just its own local correctness.
- follow-up obligation: nothing currently *prevents* a new call site
  from reintroducing a raw `fs::write`/`fs::copy`/`fs::rename` against a
  settings-like path other than this document and code review. If this
  class of finding recurs a fourth time, add a grep-based guard test
  (e.g. asserting `main.rs` contains no `fs::copy(` / bare
  `fs::write(&tmp` outside the three helpers themselves) rather than
  relying on review alone again.

**Affected areas**

- `tools/sentrith/src/main.rs` — `create_secure_file`,
  `replace_file_preserving_security`, `restore_file_content`,
  `roll_back_committed_replacement`, `reduce_hook_settings_for_baseline`,
  `restore_hook_settings_backup`, `hooks_install` (already followed this
  pattern before this ADR formalized it).

---

### ADR-20260823-02 — Two safe-write primitives, chosen by whether the file may need to stay broadly readable

**Status:** accepted

**Context**

ADR-20260823-01 named `create_secure_file` as *the* safe way to create a
file without following a symlink at its path. Applying that literally to
two new symlink findings (`.ai-usage/phase`, written by `baseline_start`;
`.sentrith-private/baseline-stash/HOOK_EDITS.txt`, written by both
`baseline_start` and `baseline_stop`) would have been wrong:
`create_secure_file` creates at an owner-only mode on Unix and an
owner/SYSTEM/Administrators-only DACL on Windows, and this codebase
already hit the consequence of that once, for `hooks_install`'s
fresh-install case — a brand-new file's restrictive creation mode
becomes its *permanent* permissions when nothing later widens it, which
broke a sandboxed process running as a different account trying to read
the file. `.ai-usage/phase` and `HOOK_EDITS.txt` hold no secrets and may
need the same broad readability `hooks_install` needed for `settings.json`.

**Decision**

Two safe-write primitives now exist, chosen by whether the file needs
long-term permission restriction:

- `create_secure_file` / `write_secure_temp_file` — for content that
  should stay narrowly restricted (a settings backup, a reduced-settings
  temp file about to be swapped into a permission-preserving replace).
- `write_ordinary_file_without_following_a_symlink` — for a file that
  must remain readable at ordinary, default/inherited permissions (a
  phase marker, a journal, anything a differently-privileged process
  might need to read). Stages through an exclusively created temp file
  at ordinary permissions and a plain rename, refusing to follow a
  symlink at either the temp or the final path exactly as
  `create_secure_file` does, without narrowing the destination.

Pick based on whether the file is meant to hold restricted/sensitive
content long-term, not by convenience or which one was reached for last.

**Rationale**

Symlink-safety and permission-restriction are two independent
properties, and this codebase's one existing primitive
(`create_secure_file`) bundled both. Using it everywhere "for safety"
would silently reintroduce the fresh-install ACL-lock class of bug at
every new call site; a second primitive that provides only the
symlink-safety property, without the permission side effect, was needed
once a second file needing broad readability showed up.

**Alternatives considered**

- Use `create_secure_file` everywhere and have callers explicitly widen
  permissions afterward (mirroring `copy_file_permissions` after
  `write_secure_temp_file` for an *existing* file) — rejected: there is
  no "original" file to copy a mode from for a brand-new marker/journal,
  which is exactly the scenario that already broke `hooks_install` once.
- Keep hand-rolling the ordinary-permission exclusive-create-plus-rename
  block per call site (as `hooks_install`'s fresh-install branch
  originally did) — rejected: this is the same duplication
  ADR-20260823-01 exists to stop; extracted into
  `write_ordinary_file_without_following_a_symlink` and reused by
  `hooks_install` itself, `baseline_start`'s phase marker and initial
  journal write, and `baseline_stop`'s retry-time journal rewrite.

**Consequences**

- positive: four call sites across `hooks_install`, `baseline_start`,
  and `baseline_stop` now share one implementation for this shape
  instead of four independently-drifting copies.
- follow-up obligation: when a new marker/journal/state file is added to
  this codebase, decide *up front* which of the two primitives it needs
  (does it hold anything requiring restriction, and does anything other
  than this process need to read it?) rather than defaulting to whichever
  one is closer in the file at the time.

**Affected areas**

- `tools/sentrith/src/main.rs` — `write_ordinary_file_without_following_a_symlink`,
  `hooks_install`, `baseline_start`, `baseline_stop`.

---

No further decisions have been recorded yet.
