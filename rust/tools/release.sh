#!/usr/bin/env bash
# Builds FlowTrace Books (the `desktop` crate) in release for the machine it runs on and packs
#   rust/dist/flowtrace-books-<version>-<os>-<arch>.tar.gz   (binary, INSTALL.txt)
#   rust/dist/flowtrace-books-<version>-<os>-<arch>.tar.gz.sha256
# The version is the desktop crate's (crates/desktop/Cargo.toml, from the workspace), <os> is
# linux or macos, <arch> is `uname -m` (x86_64, aarch64, arm64). No cross-compiling.
#
# Usage, from anywhere:
#   rust/tools/release.sh          build, strip, pack, check the archive
#   rust/tools/release.sh --name   print the archive name only (no build)
#
# It never tags, publishes or uploads anything; the tag job in .github/workflows/rust.yml runs
# it and uploads rust/dist/ as a workflow artifact.
set -euo pipefail

rust_dir=$(cd "$(dirname "$0")/.." && pwd)
cd "$rust_dir"

# `cargo pkgid` prints `path+file:///.../crates/desktop#0.1.0` (or `...#desktop@0.1.0`).
pkgid=$(cargo pkgid -p desktop)
version=${pkgid##*#}
version=${version##*@}

case "$(uname -s)" in
  Linux) os=linux ;;
  Darwin) os=macos ;;
  *) echo "release.sh: unsupported OS $(uname -s)" >&2; exit 1 ;;
esac
arch=$(uname -m)
name="flowtrace-books-${version}-${os}-${arch}"

if [ "${1:-}" = "--name" ]; then
  echo "${name}.tar.gz"
  exit 0
fi
if [ $# -gt 0 ]; then
  echo "release.sh: unknown argument $1 (only --name)" >&2
  exit 2
fi

start=$(date +%s)
cargo build --release --locked -p desktop

stage="target/release-stage/${name}"
rm -rf "$stage"
mkdir -p "$stage" dist
cp target/release/desktop "$stage/flowtrace-books"
# Symbols only; the release profile keeps no debug info. macOS strip needs -x for an executable
# that still has to link at run time.
if [ "$os" = macos ]; then strip -x "$stage/flowtrace-books"; else strip "$stage/flowtrace-books"; fi

cat > "$stage/INSTALL.txt" <<EOF
FlowTrace Books ${version} (${os}, ${arch})

Run it:
  tar xzf ${name}.tar.gz
  ./${name}/flowtrace-books

With no options and no config file it opens the demo books (Demo NGO Kenya) in memory;
nothing is saved.
To open the books on a SpacetimeDB server instead:
  ./${name}/flowtrace-books --server https://maincloud.spacetimedb.com --database flowtrace-ledger-dev
or put server, database and (optionally) token_file in
  Linux: ~/.config/flowtrace-books/config.toml
  macOS: ~/Library/Application Support/flowtrace-books/config.toml
A new identity is a member of no organisation until an operator adds it.

Other options: --theme light|dark, --entry 1-7, --screen books|connect, --report <id>.
Keys 1 to 7 switch screens.

Linux needs a Vulkan driver and a Wayland or X11 session. The binary links libssl 3, libxcb,
libxkbcommon (with its x11 part), zlib, zstd and brotli, and loads libvulkan and
libwayland-client at run time. On Debian or Ubuntu (libssl3t64 in place of libssl3 on 24.04):
  sudo apt-get install libssl3 libxcb1 libxkbcommon0 libxkbcommon-x11-0 zlib1g libzstd1 \\
    libbrotli1 libvulkan1 mesa-vulkan-drivers libwayland-client0
On macOS the archive runs on the architecture in its name only. The binary is not signed or
notarised: the first start needs right-click, Open (or xattr -d com.apple.quarantine on it).

Source and documentation: https://github.com/stchakwdev/FlowTraceNeo (rust/README.md).
EOF

archive="dist/${name}.tar.gz"
tar -C target/release-stage -czf "$archive" "$name"
if command -v sha256sum >/dev/null; then
  (cd dist && sha256sum "${name}.tar.gz" > "${name}.tar.gz.sha256")
else
  (cd dist && shasum -a 256 "${name}.tar.gz" > "${name}.tar.gz.sha256")
fi

# Check what was packed: exactly the two files, and the binary starts and parses its arguments
# (a missing option value exits 1 before any window opens, so this runs headless).
listing=$(tar -tzf "$archive" | LC_ALL=C sort | tr '\n' ' ')
expected="${name}/ ${name}/INSTALL.txt ${name}/flowtrace-books "
if [ "$listing" != "$expected" ]; then
  echo "release.sh: archive holds '$listing', expected '$expected'" >&2
  exit 1
fi
if smoke=$("$stage/flowtrace-books" --theme 2>&1); then
  echo "release.sh: '--theme' with no value should fail" >&2
  exit 1
fi
case "$smoke" in
  *"--theme needs a value"*) ;;
  *) echo "release.sh: unexpected smoke output: $smoke" >&2; exit 1 ;;
esac

elapsed=$(( $(date +%s) - start ))
bin_bytes=$(wc -c < "$stage/flowtrace-books" | tr -d ' ')
tar_bytes=$(wc -c < "$archive" | tr -d ' ')
echo "release.sh: $archive"
echo "release.sh: binary ${bin_bytes} bytes stripped, archive ${tar_bytes} bytes, ${elapsed} s"
