#!/usr/bin/env sh
set -eu

SOURCE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
FORCE=0
UPDATE=0
TARGET_DIR="."
# Backups `copy_path` kept because they held files Sentrith does not ship
# (see the comment there). Reported at the end so an update never silently
# leaves an untracked directory behind, and never silently deletes one.
KEPT_BACKUPS=""

for arg in "$@"; do
  case "$arg" in
    --force) FORCE=1 ;;
    --update) UPDATE=1 ;;
    *) TARGET_DIR="$arg" ;;
  esac
done

if [ ! -d "$TARGET_DIR" ]; then
  echo "Target directory does not exist: $TARGET_DIR" >&2
  exit 1
fi

# Contract paths are owned by Sentrith and are replaced on update.
CONTRACT_FILE=$(mktemp)
# Seed paths are owned by the project after installation. They are created when
# missing and never overwritten, so project memory survives an update.
SEED_FILE=$(mktemp)
trap 'rm -f "$CONTRACT_FILE" "$SEED_FILE"' EXIT HUP INT TERM

cat > "$CONTRACT_FILE" <<'EOF'
AGENTS.md
CLAUDE.md
.github/copilot-instructions.md
.github/prompts
.agents
.claude/skills
.claude/settings.hooks.example.json
.codex/hooks.example.json
docs/ai/BOOTSTRAP.md
docs/ai/MEMORY_POLICY.md
docs/ai/MEMORY_AUDIT.md
docs/ai/PRIVATE_CONTEXT.md
docs/ai/TASK_PROTOCOL.md
docs/ai/TASK_CLOSEOUT.md
docs/development
docs/profiles
docs/specs/README.md
docs/specs/_templates
docs/rfcs
EOF

cat > "$SEED_FILE" <<'EOF'
docs/ai/PROJECT.md
docs/ai/STATE.md
docs/ai/PROFILE.md
docs/ai/DECISIONS.md
docs/ai/KNOWN_ISSUES.md
EOF

copy_path() {
  src="$SOURCE_DIR/$1"
  dst="$TARGET_DIR/$1"
  [ -e "$src" ] || return 0
  mkdir -p "$(dirname "$dst")"
  if [ -d "$src" ]; then
    # A symlinked contract directory (dotfile managers, a shared checkout)
    # is refused rather than replaced, matching what the file branch below
    # already does. Replacing it would silently swap a managed link for a
    # real directory, and -- once the backup is removed -- destroy the only
    # record of where it pointed.
    if [ -L "$dst" ]; then
      echo "Refusing to replace symlinked directory: $dst (remove it before updating the contract)" >&2
      return 2
    fi
    # Build the replacement completely beside the destination. A failed or
    # interrupted recursive copy therefore leaves the live directory intact.
    stage_dir=$(mktemp -d "$(dirname "$dst")/.sentrith-update-stage.XXXXXX")
    if ! cp -R "$src"/. "$stage_dir"/; then
      rm -rf "$stage_dir"
      return 1
    fi
    backup_dir=""
    if [ -e "$dst" ]; then
      backup_dir=$(mktemp -d "$(dirname "$dst")/.sentrith-update-backup.XXXXXX")
      rmdir "$backup_dir"
      if ! mv "$dst" "$backup_dir"; then
        rm -rf "$stage_dir"
        return 1
      fi
    fi
    if ! mv "$stage_dir" "$dst"; then
      if [ -n "$backup_dir" ]; then
        if ! mv "$backup_dir" "$dst"; then
          echo "Directory update failed and rollback also failed; original remains at $backup_dir" >&2
        fi
      fi
      rm -rf "$stage_dir"
      return 1
    fi
    # Contract paths are Sentrith-owned and replaced wholesale, so the
    # backup is redundant *for the files Sentrith itself ships* -- but not
    # for anything else that was living under the same directory. This
    # project's own docs tell users to create files inside contract
    # directories (`docs/rfcs/`, custom `.claude/skills/`), and those may be
    # uncommitted. Deleting the backup unconditionally would destroy them
    # with no message; keeping it unconditionally accumulates an untracked
    # directory on every update, which is what this replaced. So: remove it
    # only when it holds nothing the new copy does not, and otherwise keep
    # it and say so.
    #
    # An `if`, not `[ -n "$backup_dir" ] && rm -rf ...`: under `set -e`,
    # that form's own exit status is the *test's* whenever it's false (the
    # common case, nothing to remove) -- and as the last statement in this
    # branch, that becomes copy_path's own return status, aborting the
    # whole script on every path that never needed a backup in the first
    # place. Confirmed by reproducing it: install.sh silently stopped
    # partway through the manifest with exit 1 and no error message.
    if [ -n "$backup_dir" ]; then
      extra=""
      # Compare by relative path: anything present in the backup but not in
      # the freshly installed copy did not come from Sentrith.
      while IFS= read -r rel; do
        [ -n "$rel" ] || continue
        if [ ! -e "$dst/$rel" ]; then
          extra="$extra
    $1/$rel"
        fi
      done <<EOF
$(cd "$backup_dir" && find . -type f -o -type l | sed 's|^\./||')
EOF
      if [ -n "$extra" ]; then
        KEPT_BACKUPS="$KEPT_BACKUPS
  $backup_dir  (contains:$extra
  )"
      else
        rm -rf "$backup_dir"
      fi
    fi
  else
    if [ -L "$dst" ]; then
      echo "Refusing to overwrite symlink: $dst (remove it before installing the contract file)" >&2
      return 2
    fi
    # Never copy directly onto the live contract file: an interrupted copy
    # can truncate it and leave no recoverable version. Stage beside the
    # destination (so the final move stays on one filesystem), preserve the
    # source mode, and only replace the live entry after the copy succeeds.
    tmp_file=$(mktemp "$(dirname "$dst")/.sentrith-install-file.XXXXXX")
    if ! cp -p "$src" "$tmp_file"; then
      rm -f "$tmp_file"
      return 1
    fi
    if ! mv -f "$tmp_file" "$dst"; then
      rm -f "$tmp_file"
      return 1
    fi
  fi
}

if [ "$UPDATE" -eq 1 ]; then
  while IFS= read -r path; do
    copy_path "$path"
  done < "$CONTRACT_FILE"

  added=""
  while IFS= read -r path; do
    if [ ! -e "$TARGET_DIR/$path" ]; then
      copy_path "$path"
      added="$added
  $path"
    fi
  done < "$SEED_FILE"

  printf '%s\n' "Sentrith contract updated in: $TARGET_DIR"
  printf '%s\n' "Project-owned memory was not modified."
  if [ -n "$added" ]; then
    printf '%s\n' "New memory files added as uninitialized templates:$added"
    printf '%s\n' "Ask your agent to fill them (see docs/ai/BOOTSTRAP.md)."
  fi
  if [ -n "$KEPT_BACKUPS" ]; then
    printf '%s\n' \
      "Kept backups of replaced contract directories that held files Sentrith does not ship:$KEPT_BACKUPS" \
      "Move anything you still need out of them, then delete them."
  fi
  printf '%s\n' \
    "Review: git diff -- AGENTS.md docs/development docs/profiles" \
    "Post-update steps: docs/guide/UPDATING.en.md (日本語: UPDATING.ja.md) in the Sentrith source."
  exit 0
fi

# Fresh install: refuse to clobber anything already present.
if [ "$FORCE" -ne 1 ]; then
  conflicts=""
  for listfile in "$CONTRACT_FILE" "$SEED_FILE"; do
    while IFS= read -r path; do
      if [ -e "$TARGET_DIR/$path" ]; then
        conflicts="$conflicts
  $path"
      fi
    done < "$listfile"
  done
  if [ -n "$conflicts" ]; then
    printf '%s\n' "Sentrith install stopped: target already contains:$conflicts" >&2
    echo "If Sentrith is already installed, run with --update instead: it replaces contract files and preserves project memory." >&2
    echo "Use --force only when full replacement is intentional; it overwrites project memory." >&2
    exit 2
  fi
fi

for listfile in "$CONTRACT_FILE" "$SEED_FILE"; do
  while IFS= read -r path; do
    copy_path "$path"
  done < "$listfile"
done

printf '%s\n' \
  "Sentrith repository contract copied to: $TARGET_DIR" \
  "Next:" \
  "1. Ask your coding agent to read docs/ai/BOOTSTRAP.md and bootstrap Project Memory." \
  "2. Review docs/ai/PROJECT.md, docs/ai/STATE.md, and docs/ai/PROFILE.md once." \
  "3. Optional: ./scripts/get-sentrith.sh $TARGET_DIR  (downloads the prebuilt CLI into bin/)" \
  "4. Run 'sentrith preflight' if the CLI is installed."
