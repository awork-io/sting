#!/bin/sh

set -eu

help() {
    cat <<'EOF'
Install a binary release of Sting from GitHub

Usage:
    install.sh [options]

Options:
    -h, --help      Display this message
    -f, --force     Force overwriting an existing binary
    --crate NAME    Name of the crate to install (default: sting)
    --git SLUG      GitHub repository slug (default: awork-io/sting)
    --tag TAG       Tag (version) to install (default: latest release)
    --target TARGET Target triple to install (default: rustc host)
    --to LOCATION   Installation directory (default: ~/.cargo/bin)
EOF
}

say() {
    printf 'install.sh: %s\n' "$1"
}

say_err() {
    say "$1" >&2
}

cleanup() {
    if [ -n "${td:-}" ] && [ -d "$td" ]; then
        rm -rf "$td"
    fi
}

err() {
    cleanup
    say_err "ERROR $1"
    exit 1
}

need() {
    if ! command -v "$1" >/dev/null 2>&1; then
        err "need $1 (command not found)"
    fi
}

force=false
git="awork-io/sting"
crate="sting"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --crate)
            crate="$2"
            shift
            ;;
        --force|-f)
            force=true
            ;;
        --git)
            git="$2"
            shift
            ;;
        --help|-h)
            help
            exit 0
            ;;
        --tag)
            tag="$2"
            shift
            ;;
        --target)
            target="$2"
            shift
            ;;
        --to)
            dest="$2"
            shift
            ;;
        *)
            err "unknown option: $1"
            ;;
    esac
    shift
done

need curl
need install
need mkdir
need mktemp
need tar

if [ -z "${tag:-}" ]; then
    need jq
fi

if [ -z "${target:-}" ]; then
    need cut
    need grep
    need rustc
fi

repo_url="https://github.com/$git"
releases_url="$repo_url/releases"

say_err "GitHub repository: $repo_url"
say_err "Crate: $crate"

if [ -z "${tag:-}" ]; then
    api_url="https://api.github.com/repos/$git/releases/latest"
    tag=$(curl -fsSL "$api_url" | jq -r '.tag_name')
    if [ -z "$tag" ] || [ "$tag" = "null" ]; then
        err "unable to resolve latest release tag from $api_url"
    fi
    say_err "Tag: latest ($tag)"
else
    say_err "Tag: $tag"
fi

if [ -z "${target:-}" ]; then
    target=$(rustc -Vv | grep '^host:' | cut -d' ' -f2)
fi
say_err "Target: $target"

if [ -z "${dest:-}" ]; then
    dest="$HOME/.cargo/bin"
fi
say_err "Installing to: $dest"

artifact_url="$releases_url/download/$tag/$crate-$tag-$target.tar.gz"
say_err "Artifact url: $artifact_url"

td=$(mktemp -d 2>/dev/null || mktemp -d -t sting-install)
trap cleanup EXIT

archive="$td/$crate.tar.gz"
curl -fsSL "$artifact_url" -o "$archive"
tar -C "$td" -xzf "$archive"

binary_path=""
for candidate in "$td"/*/"$crate" "$td"/"$crate"; do
    if [ -x "$candidate" ]; then
        binary_path="$candidate"
        break
    fi
done

if [ -z "$binary_path" ]; then
    err "could not find executable '$crate' in downloaded archive"
fi

mkdir -p "$dest"
dest_bin="$dest/$crate"

if [ -e "$dest_bin" ] && [ "$force" = "false" ]; then
    err "$crate already exists at $dest_bin (use --force to overwrite)"
fi

install -m 755 "$binary_path" "$dest_bin"
say_err "Installed $crate to $dest_bin"
