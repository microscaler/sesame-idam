#!/usr/bin/env bash
# This script generates an API client from the OpenAPI spec using OpenAPI Generator.
# Usage: ./generate-client.sh <language> <output_path> <spec_path_or_url>

set -e

LANGUAGE="$1"
OUTPUT_PATH="$2"
SPEC_LOCATION="$3"

if [[ -z "$LANGUAGE" || -z "$OUTPUT_PATH" || -z "$SPEC_LOCATION" ]]; then
  echo "Usage: $0 <language> <output_path> <openapi_spec_path_or_url>"
  exit 1
fi

INPUT_OPTION="$SPEC_LOCATION"
VOLUME_MOUNT=""

if [[ "$SPEC_LOCATION" != http* ]]; then
  # Make the spec path absolute
  if [[ "$SPEC_LOCATION" != /* ]]; then
    SPEC_LOCATION="$(pwd)/$SPEC_LOCATION"
  fi
  SPEC_DIR="$(dirname "$SPEC_LOCATION")"
  SPEC_FILE="$(basename "$SPEC_LOCATION")"
  VOLUME_MOUNT="-v $SPEC_DIR:/local"
  INPUT_OPTION="/local/$SPEC_FILE"
fi

OUT_DIR="$OUTPUT_PATH"
if [[ "$OUT_DIR" != /* ]]; then
  OUT_DIR="$(pwd)/$OUT_DIR"
fi
mkdir -p "$OUT_DIR"  # ensure it exists
OUT_DIR_NAME="$(basename "$OUT_DIR")"
OUT_DIR_PARENT="$(dirname "$OUT_DIR")"
VOLUME_MOUNT="$VOLUME_MOUNT -v $OUT_DIR_PARENT:/out"

docker run --rm $VOLUME_MOUNT openapitools/openapi-generator-cli:v6.6.0 generate \
  -g "$LANGUAGE" \
  -i "$INPUT_OPTION" \
  -o "/out/$OUT_DIR_NAME"
