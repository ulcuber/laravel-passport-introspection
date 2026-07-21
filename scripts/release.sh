#!/bin/bash

VERSION=$(grep -Po '(?<=^version = ")[\d\.]+' Cargo.toml)

if [[ -z "$VERSION" ]]; then
    echo "Cannot get version from Cargo.toml"
    exit 1
else
    echo "Current version: $VERSION"
fi


cargo fmt
cargo test --test integration_test --features write-tokens || exit 1

cargo build --release || exit 1

git tag "v$VERSION"
