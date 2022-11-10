#!/bin/bash

set -evo pipefail

# utility script to generate manifest file given a release

BASE_DIR="$1"
TEMPLATE_PATH="$2"
VERSION="$3"

HEAD_URL="https://github.com/scriptonist/hasura-cli-track/releases/download/${VERSION}";

LINUX_AMD64_ASSET="hasura-cli-track-linux-amd64.tar.gz" 
MACOS_AMD64_ASSET="hasura-cli-track-macos-amd64.tar.gz"
WINDOWS_AMD64_ASSET="hasura-cli-track-windows-amd64.exe.zip"

LINUX_AMD64_ASSET_LOCATION="${BASE_DIR}/hasura-cli-track-linux-amd64.tar.gz" 
MACOS_AMD64_ASSET_LOCATION="${BASE_DIR}/hasura-cli-track-macos-amd64.tar.gz"
WINDOWS_AMD64_ASSET_LOCATION="${BASE_DIR}/hasura-cli-track-windows-amd64.exe.zip"

LINUX_AMD64_ASSET_SHA256=$(cat "${LINUX_AMD64_ASSET_LOCATION}".sha256 | awk '{print $1}')
MACOS_AMD64_ASSET_SHA256=$(cat "${MACOS_AMD64_ASSET_LOCATION}".sha256 | awk '{print $1}')
WINDOWS_AMD64_ASSET_SHA256=$(cat "${WINDOWS_AMD64_ASSET_LOCATION}".sha256 | awk '{print $1}')


yq "
  .version = \"${VERSION}\" |

  .platforms[0].uri = \"$HEAD_URL/$LINUX_AMD64_ASSET\" |
  .platforms[0].sha256 = \"$LINUX_AMD64_ASSET_SHA256\" |

  .platforms[1].uri = \"$HEAD_URL/$MACOS_AMD64_ASSET\" |
  .platforms[1].sha256 = \"$MACOS_AMD64_ASSET_SHA256\" |

  .platforms[2].uri = \"$HEAD_URL/$WINDOWS_AMD64_ASSET\" |
  .platforms[2].sha256 = \"$WINDOWS_AMD64_ASSET_SHA256\" 
" "$TEMPLATE_PATH" > manifest.yaml


