#!/usr/bin/env sh
# Download the prebuilt sentrith binary for this platform into <target>/bin/,
# verifying it against the release's SHA256SUMS.
#
# Usage:
#   ./scripts/get-sentrith.sh [target-dir] [--tag sentrith-vX.Y.Z] [--repo owner/name]
#
# Defaults: target-dir=. , repo=OptimisticPessimist/sentrith, tag=newest sentrith-v* release.
# Requires: curl, sha256sum or shasum. No other dependencies.

set -eu

TARGET="."
REPO="${SENTRITH_REPO:-OptimisticPessimist/sentrith}"
TAG=""

while [ $# -gt 0 ]; do
  case "$1" in
    --tag) TAG="$2"; shift 2 ;;
    --repo) REPO="$2"; shift 2 ;;
    *) TARGET="$1"; shift ;;
  esac
done

case "$(uname -s)" in
  Linux)
    case "$(uname -m)" in
      x86_64) ASSET="sentrith-linux-x86_64" ;;
      aarch64|arm64) ASSET="sentrith-linux-aarch64" ;;
      *) echo "SENTRITH-GET: unsupported Linux arch: $(uname -m)" >&2; exit 2 ;;
    esac
    BIN_NAME="sentrith"
    ;;
  Darwin)
    case "$(uname -m)" in
      x86_64) ASSET="sentrith-macos-x86_64" ;;
      arm64) ASSET="sentrith-macos-aarch64" ;;
      *) echo "SENTRITH-GET: unsupported macOS arch: $(uname -m)" >&2; exit 2 ;;
    esac
    BIN_NAME="sentrith"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    ASSET="sentrith-windows-x86_64.exe"
    BIN_NAME="sentrith.exe"
    ;;
  *)
    echo "SENTRITH-GET: unsupported platform: $(uname -s)" >&2; exit 2 ;;
esac

if [ -z "$TAG" ]; then
  TAG="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases?per_page=30" \
    | grep -o '"tag_name": *"sentrith-v[^"]*"' \
    | head -1 \
    | sed 's/.*"\(sentrith-v[^"]*\)"/\1/')"
  if [ -z "$TAG" ]; then
    echo "SENTRITH-GET: no sentrith-v* release found in ${REPO}" >&2; exit 2
  fi
fi

BASE="https://github.com/${REPO}/releases/download/${TAG}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "SENTRITH-GET: downloading ${ASSET} from ${TAG}"
curl -fsSL -o "${TMP}/${ASSET}" "${BASE}/${ASSET}"
curl -fsSL -o "${TMP}/SHA256SUMS" "${BASE}/SHA256SUMS"

cd "$TMP"
EXPECTED="$(grep " ${ASSET}\$" SHA256SUMS | awk '{print $1}')"
if [ -z "$EXPECTED" ]; then
  echo "SENTRITH-GET: ${ASSET} not listed in SHA256SUMS" >&2; exit 2
fi
if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL="$(sha256sum "${ASSET}" | awk '{print $1}')"
else
  ACTUAL="$(shasum -a 256 "${ASSET}" | awk '{print $1}')"
fi
if [ "$EXPECTED" != "$ACTUAL" ]; then
  echo "SENTRITH-GET: checksum mismatch for ${ASSET}; refusing to install" >&2; exit 2
fi
cd - >/dev/null

mkdir -p "${TARGET}/bin"
cp "${TMP}/${ASSET}" "${TARGET}/bin/${BIN_NAME}"
chmod +x "${TARGET}/bin/${BIN_NAME}"
echo "SENTRITH-GET: installed ${TARGET}/bin/${BIN_NAME} (${TAG}, sha256 verified)"
