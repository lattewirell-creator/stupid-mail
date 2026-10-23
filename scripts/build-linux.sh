#!/bin/sh
# Builds The Stupid Mail for Linux and packages it as release-<version>/StupidMail-<version>-linux-amd64.deb.
# Needs Rust, cargo-deb (cargo install cargo-deb) and: libwebkit2gtk-4.1-dev libgtk-3-dev libssl-dev libdbus-1-dev.
# The "-linux-amd64.deb" ending matters: Stupid OS finds the Linux package in each GitHub release by it.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root/windows"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
out="$root/release-$version/StupidMail-$version-linux-amd64.deb"
mkdir -p "$root/release-$version"
cargo test --release
cargo deb --output "$out"
dpkg-deb --info "$out"
echo "$out"
