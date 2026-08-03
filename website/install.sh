#!/bin/sh
set -eu

BIN="shepherd"
MANIFEST_URL="https://shepherd.dev/shepherd-latest.json"
INSTALL_DIR="${SHEPHERD_INSTALL_DIR:-$HOME/.local/bin}"

main() {
    echo ""
    echo "      ,ww"
    echo "     wWWWWWWW_)  shepherd installer"
    echo "     \`WWWWWW'    shepherd.dev"
    echo "      II  II"
    echo ""

    OS="$(uname -s)"
    case "$OS" in
        Linux) os="linux" ;;
        Darwin) os="macos" ;;
        *) err "unsupported OS: $OS" ;;
    esac

    ARCH="$(uname -m)"
    case "$ARCH" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *) err "unsupported architecture: $ARCH" ;;
    esac

    need curl
    need awk

    TARGET="${os}-${arch}"
    MANIFEST="$(curl -fsSL --proto '=https' --proto-redir '=https' --retry 3 --connect-timeout 10 --max-time 20 "$MANIFEST_URL")" \
        || err "can't reach ${MANIFEST_URL}. Please try again later; shepherd.dev might be down. Who let the sheeps out? baaa."
    URL="$(asset_field "$MANIFEST" "$TARGET" "url")"
    SHA256="$(asset_field "$MANIFEST" "$TARGET" "sha256")"
    VERSION="$(printf '%s\n' "$MANIFEST" | awk -F '"' '/^[[:space:]]*"version"[[:space:]]*:/ { print $4; exit }')"

    [ -n "$URL" ] || err "no Shepherd binary release exists for ${TARGET}; use the installfest-owned shepherd-install source build"
    [ -n "$SHA256" ] || err "release manifest does not include a sha256 for ${TARGET}"

    case "$URL" in
        https://*) ;;
        *) err "release manifest provided non-https asset URL for ${TARGET}" ;;
    esac

    case "$SHA256" in
        [0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]) ;;
        *) err "release manifest provided an invalid sha256 for ${TARGET}" ;;
    esac

    if [ -n "$VERSION" ]; then
        log "downloading ${BIN} v${VERSION} for ${TARGET}"
    else
        log "downloading ${BIN} for ${TARGET}"
    fi

    TMP="$(mktemp -d)"
    trap 'rm -rf "$TMP"' EXIT

    if ! curl -fsSL --proto '=https' --proto-redir '=https' --retry 3 --connect-timeout 10 --max-time 120 "$URL" -o "${TMP}/${BIN}"; then
        err "download failed from ${URL}"
    fi

    ACTUAL_SHA256="$(checksum_file "${TMP}/${BIN}")"
    [ "$ACTUAL_SHA256" = "$SHA256" ] || err "download checksum mismatch for ${TARGET}"

    chmod +x "${TMP}/${BIN}"
    mkdir -p "${INSTALL_DIR}"
    mv "${TMP}/${BIN}" "${INSTALL_DIR}/${BIN}"

    if [ -n "$VERSION" ]; then
        log "installed ${BIN} v${VERSION} to ${INSTALL_DIR}/${BIN}"
    else
        log "installed ${BIN} to ${INSTALL_DIR}/${BIN}"
    fi

    if ! in_path "${INSTALL_DIR}"; then
        warn "${INSTALL_DIR} is not in your PATH"
        echo "  add it to your shell config:"
        echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
    fi

    if command -v "$BIN" >/dev/null 2>&1; then
        log "run '${BIN}' to launch Shepherd"
    fi
}

log() { printf '  \033[32m>\033[0m %s\n' "$1"; }
warn() { printf '  \033[33m!\033[0m %s\n' "$1"; }
err() { printf '  \033[31m✗\033[0m %s\n' "$1" >&2; exit 1; }

asset_field() {
    manifest="$1"
    target="$2"
    field="$3"
    printf '%s\n' "$manifest" | awk -v target="\"${target}\"" -v field="\"${field}\"" '
        /^[[:space:]]*"assets"[[:space:]]*:/ { in_assets = 1; next }
        in_assets && !in_target && /^[[:space:]]*}/ { exit }
        in_assets && !in_target && index($0, target) { in_target = 1; next }
        in_target && /^[[:space:]]*}/ { exit }
        in_target && index($0, field) {
            if (match($0, /"[^"]+"[[:space:]]*:[[:space:]]*"([^"]+)"/, m)) {
                print m[1]
                exit
            }
        }
    '
}

checksum_file() {
    path="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$path" | awk '{print $1}'
        return
    fi
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$path" | awk '{print $1}'
        return
    fi
    err "requires 'sha256sum' or 'shasum' — install one first, or download a binary manually from https://shepherd.dev/docs/install/"
}

need() {
    if ! command -v "$1" >/dev/null 2>&1; then
        err "requires '$1' — install it first, or download a binary manually from https://shepherd.dev/docs/install/"
    fi
}

in_path() {
    case ":$PATH:" in
        *":$1:"*) return 0 ;;
        *) return 1 ;;
    esac
}

main "$@"
