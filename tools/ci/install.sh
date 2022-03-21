#!/bin/sh

# https://github.com/japaric/trust/blob/6df66ab2b4a8e174e00ec322512288f19d3dae5a/tools/ci/install.sh
# Copyright (c) 2016 Jorge Aparicio
# Modified (c) 2021 Leigh Johnson

# Permission is hereby granted, free of charge, to any
# person obtaining a copy of this software and associated
# documentation files (the "Software"), to deal in the
# Software without restriction, including without
# limitation the rights to use, copy, modify, merge,
# publish, distribute, sublicense, and/or sell copies of
# the Software, and to permit persons to whom the Software
# is furnished to do so, subject to the following
# conditions:

# The above copyright notice and this permission notice
# shall be included in all copies or substantial portions
# of the Software.

# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
# ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
# TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
# PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
# SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
# CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
# OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
# IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
# DEALINGS IN THE SOFTWARE.
#!/bin/sh

set -e

help() {
    cat <<'EOF'
Install PrintNanny Command-line Interface

Usage:
    install.sh [options]

Options:
    -h, --help      Display this message
    -f, --force     Force overwriting an existing binary
    --tag TAG       Tag (version) of the crate to install (default <latest release>)
    --target TARGET Install the release compiled for $TARGET (default <`rustc` host>)
    --to LOCATION   Where to install the binary (default ~/.cargo/bin)
EOF
}

say() {
    echo "install.sh: $1"
}

say_err() {
    say "$1" >&2
}

err() {
    if [ ! -z $td ]; then
        rm -rf $td
    fi

    say_err "ERROR $1"
    exit 1
}

need() {
    if ! command -v $1 > /dev/null 2>&1; then
        err "need $1 (command not found)"
    fi
}

force=false
while test $# -gt 0; do
    case $1 in
        --force | -f)
            force=true
            ;;
        --help | -h)
            help
            exit 0
            ;;
        --tag)
            tag=$2
            shift
            ;;
        --target)
            target=$2
            shift
            ;;
        --to)
            dest=$2
            shift
            ;;
        *)
            ;;
    esac
    shift
done

# Dependencies
need basename
need curl
need install
need mkdir
need mktemp
need tar

# Optional dependencies
if [ -z $crate ] || [ -z $tag ] || [ -z $target ]; then
    need cut
fi

if [ -z $tag ]; then
    need rev
fi

if [ -z $target ]; then
    need grep
    need rustc
fi

if [ -z $git ]; then
    err 'must specify a git repository using `--git`. Example: `install.sh --git japaric/cross`'
fi

url="https://github.com/bitsy-ai/print-nanny-cli"
crate="printnanny"
say_err "GitHub repository: $url"

if [ -z $crate ]; then
    crate=$(echo $git | cut -d'/' -f2)
fi

say_err "Crate: $crate"

url="$url/releases"

if [ -z $tag ]; then
    tag=$(curl -s "$url/latest" | cut -d'"' -f2 | rev | cut -d'/' -f1 | rev)
    say_err "Tag: latest ($tag)"
else
    say_err "Tag: $tag"
fi

if [ -z $target ]; then
    target=$(rustc -Vv | grep host | cut -d' ' -f2)
fi

say_err "Target: $target"

if [ -z $dest ]; then
    dest="$HOME/.cargo/bin"
fi

say_err "Installing to: $dest"

url="$url/download/$tag/$crate-$tag-$target.tar.gz"

td=$(mktemp -d || mktemp -d -t tmp)
curl -sL $url | tar -C $td -xz

for f in $(ls $td); do
    test -x $td/$f || continue

    if [ -e "$dest/$f" ] && [ $force = false ]; then
        err "$f already exists in $dest"
    else
        mkdir -p $dest
        install -m 755 $td/$f $dest
    fi
done

trap "rm -rf $td" EXIT
