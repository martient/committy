#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
image_name="committy-native-git-e2e"

docker build \
  --file "$repo_root/tools/native-git-e2e/Dockerfile" \
  --tag "$image_name" \
  "$repo_root"

docker run --rm --tty "$image_name"
