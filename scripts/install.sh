#!/usr/bin/env sh
set -eu

SOURCE_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
FORCE=0
TARGET_DIR="."

for arg in "$@"; do
  case "$arg" in
    --force) FORCE=1 ;;
    *) TARGET_DIR="$arg" ;;
  esac
done

if [ ! -d "$TARGET_DIR" ]; then
  echo "Target directory does not exist: $TARGET_DIR" >&2
  exit 1
fi

PATHS_FILE=$(mktemp)
trap 'rm -f "$PATHS_FILE"' EXIT HUP INT TERM
cat > "$PATHS_FILE" <<'EOF'
AGENTS.md
CLAUDE.md
.github/copilot-instructions.md
.github/prompts
.claude
.codex
docs/ai
docs/development
docs/specs
docs/rfcs
EOF

if [ "$FORCE" -ne 1 ]; then
  conflicts=""
  while IFS= read -r path; do
    if [ -e "$TARGET_DIR/$path" ]; then
      conflicts="$conflicts
  $path"
    fi
  done < "$PATHS_FILE"
  if [ -n "$conflicts" ]; then
    printf '%s\n' "Sentrith install stopped: target already contains:$conflicts" >&2
    echo "Review/merge those files manually, or rerun with --force only if replacement is intentional." >&2
    exit 2
  fi
fi

copy_path() {
  src="$SOURCE_DIR/$1"
  dst="$TARGET_DIR/$1"
  [ -e "$src" ] || return 0
  mkdir -p "$(dirname "$dst")"
  if [ -d "$src" ]; then
    mkdir -p "$dst"
    cp -R "$src"/. "$dst"/
  else
    cp "$src" "$dst"
  fi
}

while IFS= read -r path; do
  copy_path "$path"
done < "$PATHS_FILE"

printf '%s\n' \
  "Sentrith repository contract copied to: $TARGET_DIR" \
  "Next:" \
  "1. Ask your coding agent to read docs/ai/BOOTSTRAP.md and bootstrap Project Memory." \
  "2. Review docs/ai/PROJECT.md and docs/ai/STATE.md once." \
  "3. Run 'sentrith preflight' if the CLI is installed."
