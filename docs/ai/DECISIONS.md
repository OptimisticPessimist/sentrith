# Engineering Decisions

> Durable decisions whose rationale would otherwise be expensive to rediscover.
> This is not a changelog.

## Entry template

### ADR-YYYYMMDD-NN — Short decision title

**Status:** accepted | superseded | deprecated

**Context**

What problem or constraint required a decision?

**Decision**

What was chosen?

**Rationale**

Why was this chosen for this repository?

**Alternatives considered**

- Alternative A — why rejected
- Alternative B — why rejected

**Consequences**

- positive consequence
- tradeoff or constraint
- follow-up obligation

**Affected areas**

- `path/to/file`
- subsystem/API/schema

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

No further project-specific decisions have been recorded yet.
