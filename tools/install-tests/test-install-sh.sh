#!/bin/sh
# Exercise website/docs/install.sh against the published releases.
#
#   sh tools/install-tests/test-install-sh.sh <shell>     (<shell>: sh or bash)
#
# Needs the GitHub CLI (gh) to read the release list and to build a local
# mirror, which stands in for PYFARO_DOWNLOAD_BASE in the tamper tests.
set -eu

shell=${1:?usage: test-install-sh.sh <shell>}
here=$(cd "$(dirname "$0")" && pwd)
script=$here/../../website/docs/install.sh
repo=pyfaro/pyfaro
pinned=0.3.0-alpha.1

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

echo "== pinned version ($shell)"
PYFARO_VERSION=$pinned PYFARO_INSTALL_DIR=$work/bin "$shell" "$script"
[ "$("$work/bin/pyfaro" --version)" = "pyfaro $pinned" ] || fail "pinned version not installed"

echo "== newest release upgrades in place ($shell)"
newest=$(gh release list --repo "$repo" --limit 1 --json tagName --jq '.[0].tagName')
newest=${newest#v}
PYFARO_INSTALL_DIR=$work/bin GITHUB_TOKEN=${GITHUB_TOKEN:-${GH_TOKEN:-}} "$shell" "$script"
[ "$("$work/bin/pyfaro" --version)" = "pyfaro $newest" ] || fail "newest release $newest not installed"

echo "== a pinned version downgrades ($shell)"
PYFARO_VERSION=v$pinned PYFARO_INSTALL_DIR=$work/bin "$shell" "$script"
[ "$("$work/bin/pyfaro" --version)" = "pyfaro $pinned" ] || fail "downgrade did not happen"

echo "== local mirror ($shell)"
mirror=$work/mirror
mkdir -p "$mirror/v$pinned"
gh release download "v$pinned" --repo "$repo" --dir "$mirror/v$pinned" \
    --pattern 'pyfaro-*' --pattern SHA256SUMS
PYFARO_VERSION=$pinned PYFARO_DOWNLOAD_BASE=file://$mirror PYFARO_INSTALL_DIR=$work/mirrored "$shell" "$script"
[ "$("$work/mirrored/pyfaro" --version)" = "pyfaro $pinned" ] || fail "mirror install failed"

# Run the installer expecting a failure that installs nothing.
expect_refusal() {
    label=$1
    base=$2
    dir=$work/refused-$label
    if PYFARO_VERSION=$pinned PYFARO_DOWNLOAD_BASE=$base PYFARO_INSTALL_DIR=$dir "$shell" "$script"; then
        fail "$label: the installer succeeded"
    fi
    [ ! -e "$dir" ] || fail "$label: the installer left $dir behind"
}

echo "== corrupted archive is refused ($shell)"
cp -R "$mirror" "$work/corrupt"
for archive in "$work/corrupt/v$pinned"/*.tar.gz; do
    printf 'x' >>"$archive"
done
expect_refusal corrupt-archive "file://$work/corrupt"

echo "== wrong checksum is refused ($shell)"
cp -R "$mirror" "$work/wrongsum"
sed 's/^[0-9a-f]\{64\}/0000000000000000000000000000000000000000000000000000000000000000/' \
    "$mirror/v$pinned/SHA256SUMS" >"$work/wrongsum/v$pinned/SHA256SUMS"
expect_refusal wrong-checksum "file://$work/wrongsum"

echo "== missing checksum entry is refused ($shell)"
cp -R "$mirror" "$work/nosum"
: >"$work/nosum/v$pinned/SHA256SUMS"
expect_refusal missing-entry "file://$work/nosum"

echo "== plain HTTP mirror is refused ($shell)"
expect_refusal http "http://127.0.0.1:9"

echo "OK ($shell)"
