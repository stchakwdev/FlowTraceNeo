#!/usr/bin/env bash
# Builds FlowTrace Books (the `desktop` crate) in release for the machine it runs on and packs
#   rust/dist/flowtrace-books-<version>-<os>-<arch>.tar.gz   (binary, INSTALL.txt, config.toml)
#   rust/dist/flowtrace-books-<version>-<os>-<arch>.tar.gz.sha256
# The version is the desktop crate's (crates/desktop/Cargo.toml, from the workspace), <os> is
# linux or macos, <arch> is `uname -m` (x86_64, aarch64, arm64). No cross-compiling.
# config.toml is crates/desktop/release/config.toml unchanged: it names the production database
# (flowtrace-ledger on maincloud) for people who copy it into the app's config dir.
#
# Usage, from anywhere:
#   rust/tools/release.sh          build, strip, pack, check the archive
#   rust/tools/release.sh --name   print the archive name only (no build)
#   rust/tools/release.sh --check <archive>   run the archive content check only (no build)
#
# It never tags, publishes or uploads anything; the tag job in .github/workflows/rust.yml runs
# it and uploads rust/dist/ as a workflow artifact.
set -euo pipefail

caller_dir=$PWD
rust_dir=$(cd "$(dirname "$0")/.." && pwd)
cd "$rust_dir"

# The desktop crate inherits the workspace version (tools/src/release.test.ts checks both).
# Read from Cargo.toml, not `cargo pkgid`, so `--name` and `--check` need no Rust toolchain:
# without one installed, rustup fetches rust-toolchain.toml's channel on the first cargo call.
if ! grep -qx 'version.workspace = true' crates/desktop/Cargo.toml; then
  echo "release.sh: crates/desktop/Cargo.toml does not inherit the workspace version" >&2
  exit 1
fi
version=$(awk '/^\[/ { p = ($0 == "[workspace.package]") } p && /^version = "/ { gsub(/^version = "|"$/, ""); print; exit }' Cargo.toml)
if [ -z "$version" ]; then
  echo "release.sh: no version in [workspace.package] of Cargo.toml" >&2
  exit 1
fi

case "$(uname -s)" in
  Linux) os=linux ;;
  Darwin) os=macos ;;
  *) echo "release.sh: unsupported OS $(uname -s)" >&2; exit 1 ;;
esac
arch=$(uname -m)
name="flowtrace-books-${version}-${os}-${arch}"
config="crates/desktop/release/config.toml"

# What was packed: exactly the binary, INSTALL.txt and config.toml, the last byte for byte the
# one in the repository (crates/desktop/src/config.rs tests that it opens flowtrace-ledger).
check_archive() {
  local archive=$1 listing expected
  if ! listing=$(tar -tzf "$archive" | LC_ALL=C sort | tr '\n' ' '); then
    echo "release.sh: cannot list $archive" >&2
    return 1
  fi
  expected="${name}/ ${name}/INSTALL.txt ${name}/config.toml ${name}/flowtrace-books "
  if [ "$listing" != "$expected" ]; then
    echo "release.sh: archive holds '$listing', expected '$expected'" >&2
    return 1
  fi
  if ! tar -xOzf "$archive" "${name}/config.toml" | cmp -s - "$config"; then
    echo "release.sh: config.toml differs from $config" >&2
    return 1
  fi
}

case "${1:-}" in
  --name)
    echo "${name}.tar.gz"
    exit 0
    ;;
  --check)
    if [ $# -ne 2 ]; then
      echo "release.sh: --check takes one archive" >&2
      exit 2
    fi
    case "$2" in
      /*) check_archive "$2" ;;
      *) check_archive "$caller_dir/$2" ;;
    esac
    exit
    ;;
esac
if [ $# -gt 0 ]; then
  echo "release.sh: unknown argument $1 (only --name or --check <archive>)" >&2
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
cp "$config" "$stage/config.toml"

cat > "$stage/INSTALL.txt" <<EOF
FlowTrace Books ${version} (${os}, ${arch})

Run it:
  tar xzf ${name}.tar.gz
  ./${name}/flowtrace-books

Demo or production books:
- Without a config file it opens the demo books (Demo NGO Kenya) in memory; nothing is saved.
- To open the production books (database flowtrace-ledger on maincloud), copy the config.toml
  from this archive into the app's config directory, then start the app:
    Linux: mkdir -p ~/.config/flowtrace-books
           cp ${name}/config.toml ~/.config/flowtrace-books/
    macOS: mkdir -p ~/Library/Application\ Support/flowtrace-books
           cp ${name}/config.toml ~/Library/Application\ Support/flowtrace-books/
  Remove it from there to go back to the demo. --server URL and --database NAME win over it.
- On the first connect the app creates your token file next to config.toml
  (token-maincloud.spacetimedb.com). It is your login: keep it private, never send it.
- A new identity sees no organisation until someone adds it: an operator, or a member of the
  organisation who can close periods, adds you by your identity. The Connection screen shows
  its first eight hex digits; start with --timings on to print all 64 on stderr. How they add
  you: rust/docs/module-release.md, "Operators and members", in the source repository below.

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

# Check what was packed, and that the binary starts and parses its arguments (a missing option
# value exits 1 before any window opens, so this runs headless).
check_archive "$archive"
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
