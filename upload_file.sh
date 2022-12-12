#!/usr/bin/env bash

set -e
[ "$TRACE" ] && set -x

tag="v0.2.0"
GH_API="https://api.github.com"
GH_REPO="$GH_API/repos/iamcco/ds-pinyin-lsp"
GH_TAGS="$GH_REPO/releases/tags/$tag"
AUTH="Authorization: token $GITHUB_API_TOKEN"

# upload assets
cd ./packages/ds-pinyin-lsp/target/aarch64-apple-darwin/release/

declare -a files=("ds-pinyin-lsp_v0.2.0_aarch64-apple-darwin.zip")

# Validate token.
curl -o /dev/null -sH "$AUTH" $GH_REPO || { echo "Error: Invalid repo, token or network issue!";  exit 1; }

# Read asset tags.
response=$(curl -sH "$AUTH" $GH_TAGS)

# Get ID of the asset based on given filename.
eval $(echo "$response" | grep -m 1 "id.:" | grep -w id | tr : = | tr -cd '[[:alnum:]]=')
[ "$id" ] || { echo "Error: Failed to get release id for tag: $tag"; echo "$response" | awk 'length($0)<100' >&2; exit 1; }

# Upload asset
for filename in "${files[@]}"
do
  GH_ASSET="https://uploads.github.com/repos/iamcco/ds-pinyin-lsp/releases/$id/assets?name=$filename"
  echo "Uploading $filename"
  curl -X POST -H "Authorization: token $GITHUB_API_TOKEN" \
    -H "Content-Type: application/octet-stream" \
    --data-binary @"$filename" \
    $GH_ASSET
done

