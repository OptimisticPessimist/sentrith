#!/usr/bin/env sh
set -eu

SOURCE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
FORCE=0
UPDATE=0
TARGET_DIR="."

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
    if [ -e "$dst" ] || [ -L "$dst" ]; then
      backup_dir=$(mktemp -d "$(dirname "$dst")/.sentrith-update-backup.XXXXXX")
      rmdir "$backup_dir"
      mv "$dst" "$backup_dir"
    fi
    mkdir -p "$dst"
    cp -R "$src"/. "$dst"/
  else
    cp "$src" "$dst"
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
