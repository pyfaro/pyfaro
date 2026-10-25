#!/bin/sh
# Pyfaro installer for Linux and macOS. Published at https://pyfaro.dev/install.sh
#
#   curl --proto '=https' --tlsv1.2 -LsSf https://pyfaro.dev/install.sh | sh
#
# Prefer to read it first? Download it, inspect it, then run it:
#
#   curl --proto '=https' --tlsv1.2 -LsSf https://pyfaro.dev/install.sh -o install.sh
#   less install.sh
#   sh install.sh
#
# What it does:
#   1. picks the release archive for this OS and CPU;
#   2. downloads the archive and SHA256SUMS over HTTPS;
#   3. verifies the archive's SHA-256 and stops if it cannot or does not match;
#   4. installs the pyfaro binary and prints what it did.
# It never uses sudo, never edits shell profiles and never installs uv.
# Running it again upgrades Pyfaro in place.
#
# Environment variables:
#   PYFARO_VERSION        version to install, e.g. 0.3.0-alpha.1 (default: the
#                         newest release, prereleases included)
#   PYFARO_INSTALL_DIR    where to put the binary (default: ~/.local/bin)
#   PYFARO_DOWNLOAD_BASE  release download URL prefix, for testing mirrors only.
#                         Default: https://github.com/pyfaro/pyfaro/releases/download
#                         Must start with https:// or file:// (a local directory).
#   GITHUB_TOKEN          optional; sent to api.github.com only, to avoid rate
#                         limits when looking up the newest release.
#
# Everything below is a function and the last line calls main, so a download
# that is cut short cannot run half a script.

set -eu

REPO=pyfaro/pyfaro
DEFAULT_BASE=https://github.com/pyfaro/pyfaro/releases/download
TMP=

say() {
    printf 'pyfaro-install: %s\n' "$*"
}

die() {
    printf 'pyfaro-install: error: %s\n' "$*" >&2
    exit 1
}

cleanup() {
    if [ -n "$TMP" ]; then
        rm -rf "$TMP"
    fi
}

# The release target (Rust target triple) for this machine.
detect_target() {
    os=$(uname -s)
    arch=$(uname -m)
    case "$os" in
    Linux)
        # The Linux archives are linked against glibc.
        case $(ldd --version 2>&1 || true) in
        *musl*) die "musl-based Linux is not supported by the prebuilt archives; use: cargo install pyfaro --locked" ;;
        esac
        case "$arch" in
        x86_64 | amd64) echo x86_64-unknown-linux-gnu ;;
        aarch64 | arm64) echo aarch64-unknown-linux-gnu ;;
        *) die "unsupported CPU architecture on Linux: $arch (supported: x86_64, aarch64)" ;;
        esac
        ;;
    Darwin)
        # A shell running under Rosetta reports x86_64 on Apple silicon.
        if [ "$arch" = x86_64 ] && [ "$(sysctl -n hw.optional.arm64 2>/dev/null || true)" = 1 ]; then
            arch=arm64
        fi
        case "$arch" in
        x86_64) echo x86_64-apple-darwin ;;
        arm64 | aarch64) echo aarch64-apple-darwin ;;
        *) die "unsupported CPU architecture on macOS: $arch (supported: x86_64, arm64)" ;;
        esac
        ;;
    MINGW* | MSYS* | CYGWIN* | Windows_NT)
        die "this script is for Linux and macOS; on Windows use install.ps1: https://pyfaro.dev/getting-started/installation/"
        ;;
    *)
        die "unsupported operating system: $os (supported: Linux, macOS); see https://pyfaro.dev/getting-started/installation/"
        ;;
    esac
}

# download URL DESTINATION ("-" is standard output): HTTPS only, TLS 1.2+,
# redirects included.
download() {
    if command -v curl >/dev/null 2>&1; then
        curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL -o "$2" "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget --https-only --secure-protocol=TLSv1_2 -q -O "$2" "$1"
    else
        die "neither curl nor wget found; install one of them and run this script again"
    fi
}

# fetch NAME DESTINATION: one release asset, from the HTTPS release URL or,
# when PYFARO_DOWNLOAD_BASE is a file:// URL, from a local directory.
fetch() {
    url="$BASE/v$VERSION/$1"
    case "$BASE" in
    file://*) cp "${url#file://}" "$2" || die "could not read $url" ;;
    *) download "$url" "$2" || die "could not download $url" ;;
    esac
}

# The newest release tag, prereleases included. (/releases/latest ignores
# them and answers 404 while every release is a prerelease.)
newest_version() {
    api="https://api.github.com/repos/$REPO/releases?per_page=1"
    if [ -n "${GITHUB_TOKEN:-}" ] && command -v curl >/dev/null 2>&1; then
        json=$(curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL \
            -H "Authorization: Bearer $GITHUB_TOKEN" "$api") || json=
    else
        json=$(download "$api" -) || json=
    fi
    tag=$(printf '%s\n' "$json" |
        sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' |
        head -n 1)
    [ -n "$tag" ] || die "could not look up the newest release at $api (rate limited? set PYFARO_VERSION, for example PYFARO_VERSION=0.3.0-alpha.1)"
    echo "${tag#v}"
}

# Choose the SHA-256 tool now, so a machine without one fails before any
# download instead of installing an unverified binary.
find_sha256_tool() {
    if command -v sha256sum >/dev/null 2>&1; then
        SHA256_TOOL=sha256sum
    elif command -v shasum >/dev/null 2>&1; then
        SHA256_TOOL=shasum
    elif command -v openssl >/dev/null 2>&1; then
        SHA256_TOOL=openssl
    else
        die "no SHA-256 tool found (need sha256sum, shasum or openssl); refusing to install an unverified binary"
    fi
}

sha256_of() {
    case "$SHA256_TOOL" in
    sha256sum) sha256sum "$1" | cut -d ' ' -f 1 ;;
    shasum) shasum -a 256 "$1" | cut -d ' ' -f 1 ;;
    openssl) openssl dgst -sha256 "$1" | sed 's/.*= *//' ;;
    esac
}

# Fail unless FILE's SHA-256 is the one SHA256SUMS lists under NAME.
verify() {
    sum_file=$1
    sum_name=$2
    sum_list=$3
    expected=$(awk -v name="$sum_name" '$2 == name || $2 == "*" name { print $1; exit }' "$sum_list")
    [ -n "$expected" ] || die "SHA256SUMS has no entry for $sum_name; refusing to install"
    case "$expected" in
    *[!0-9a-fA-F]*) die "SHA256SUMS has a malformed entry for $sum_name; refusing to install" ;;
    esac
    [ "${#expected}" -eq 64 ] || die "SHA256SUMS has a malformed entry for $sum_name; refusing to install"
    actual=$(sha256_of "$sum_file")
    [ -n "$actual" ] || die "could not compute the SHA-256 of $sum_name; refusing to install"
    # Compare in lower case: tools disagree on the case of hex digits.
    expected=$(printf '%s' "$expected" | tr 'A-F' 'a-f')
    actual=$(printf '%s' "$actual" | tr 'A-F' 'a-f')
    if [ "$actual" != "$expected" ]; then
        die "checksum mismatch for $sum_name (expected $expected, got $actual); the download is corrupt or has been tampered with, nothing was installed"
    fi
}

main() {
    VERSION=${PYFARO_VERSION:-}
    VERSION=${VERSION#v}
    BASE=${PYFARO_DOWNLOAD_BASE:-$DEFAULT_BASE}
    BASE=${BASE%/}
    case "$BASE" in
    https://* | file://*) ;;
    *) die "PYFARO_DOWNLOAD_BASE must start with https:// (or file:// for a local test mirror), got: $BASE" ;;
    esac
    if [ -n "${PYFARO_INSTALL_DIR:-}" ]; then
        INSTALL_DIR=$PYFARO_INSTALL_DIR
    elif [ -n "${HOME:-}" ]; then
        INSTALL_DIR=$HOME/.local/bin
    else
        die "HOME is not set; set PYFARO_INSTALL_DIR to the directory to install into"
    fi

    target=$(detect_target)
    find_sha256_tool

    if [ -z "$VERSION" ]; then
        VERSION=$(newest_version)
    fi
    case "$VERSION" in
    '' | *[!0-9A-Za-z.+-]*) die "invalid version: $VERSION" ;;
    esac
    say "installing pyfaro $VERSION for $target"

    TMP=$(mktemp -d) || die "could not create a temporary directory"
    trap cleanup EXIT
    trap 'exit 1' HUP INT TERM

    name=pyfaro-$VERSION-$target
    archive=$name.tar.gz
    fetch "$archive" "$TMP/$archive"
    fetch SHA256SUMS "$TMP/SHA256SUMS"
    verify "$TMP/$archive" "$archive" "$TMP/SHA256SUMS"
    say "verified the SHA-256 of $archive"

    mkdir "$TMP/extract"
    tar -xzf "$TMP/$archive" -C "$TMP/extract" || die "could not extract $archive"
    [ -f "$TMP/extract/$name/pyfaro" ] || die "$archive does not contain $name/pyfaro"

    dest=$INSTALL_DIR/pyfaro
    previous=
    if [ -x "$dest" ]; then
        previous=$("$dest" --version 2>/dev/null || true)
    fi
    mkdir -p "$INSTALL_DIR" || die "could not create $INSTALL_DIR (this script never uses sudo; set PYFARO_INSTALL_DIR to a directory you own)"
    # Copy beside the destination, then rename: replaces a running binary
    # atomically and never leaves a half-written pyfaro behind.
    staged=$INSTALL_DIR/.pyfaro.$$
    if ! cp "$TMP/extract/$name/pyfaro" "$staged" || ! chmod 755 "$staged" || ! mv -f "$staged" "$dest"; then
        rm -f "$staged"
        die "could not write $dest (this script never uses sudo; set PYFARO_INSTALL_DIR to a directory you own)"
    fi
    if [ -n "$previous" ]; then
        say "replaced $previous"
    fi
    say "installed pyfaro $VERSION ($target) to $dest"

    case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        say "$INSTALL_DIR is not on your PATH. For this shell, run:"
        # shellcheck disable=SC2016 # $PATH is meant literally: it is advice to type
        printf '\n    export PATH="%s:%s"\n\n' "$INSTALL_DIR" '$PATH'
        say "and add that line to your shell profile (for example ~/.profile or ~/.zshrc) to keep it."
        ;;
    esac

    "$dest" --version

    if command -v uv >/dev/null 2>&1; then
        say "found uv: $(command -v uv); run 'pyfaro doctor' to check it is new enough (0.9.26 or newer)"
    else
        say "uv was not found on PATH. Pyfaro needs uv 0.9.26 or newer and never installs it for you."
        say "Install uv yourself, for example:"
        printf '\n    curl -LsSf https://astral.sh/uv/install.sh | sh\n\n'
        say "More options: https://docs.astral.sh/uv/getting-started/installation/"
    fi
}

main "$@"
