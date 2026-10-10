#!/bin/bash
# Builds prism-book.pdf from the chapters. Run from anywhere.
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
CH="${1:-$HERE/..}"
OUT="${2:-$HERE/../prism-book.pdf}"
pandoc "$CH"/ch*.md \
  -f markdown+pipe_tables+smart-yaml_metadata_block \
  --top-level-division=chapter \
  --lua-filter="$HERE/filter.lua" \
  --syntax-definition="$HERE/prism.xml" \
  --highlight-style=monochrome \
  --template="$HERE/template.tex" \
  --pdf-engine=xelatex \
  -V tables=true \
  -o "$OUT"
