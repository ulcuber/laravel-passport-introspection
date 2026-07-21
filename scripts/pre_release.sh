#!/bin/bash

VERSION=$(grep -Po '(?<=^version = ")[\d\.]+' Cargo.toml)

if [[ -z "$VERSION" ]]; then
    echo "Cannot get version from Cargo.toml"
    exit 1
else
    echo "Current version: $VERSION"
fi

cargo tree --depth 1 2>/dev/null | head -1

if [[ $(git tag --list "v$VERSION") ]]; then
    echo "Git version tag aleady exists"
else
    echo "Git version tag is available"
fi

if ! git diff --quiet; then
    echo "Working directory has uncommitted changes"
    git status --short
    exit 1
else
    echo "Working directory is clean"
fi

BRANCH=$(git branch --show-current)
if [[ "$BRANCH" != "main" ]]; then
    echo "Not on main branch (current: $BRANCH)"
    exit 1
fi

echo
sed -n "/## \[$VERSION\]/,/## \[/p" CHANGELOG.md | head -n -1
