#!/bin/bash

if [ $# -lt 1 ]; then
    echo "Error: Name of fuzz target is required."
    echo "Usage: $0 fuzz_target [sources...]"
    exit 1
fi

FUZZ_TARGET="$1"
shift
SRC_FILTER="$@"

OUT="fuzz/coverage/$FUZZ_TARGET/html/"

cargo fuzz coverage $FUZZ_TARGET

TARGET=$(rustc -vV | sed -n 's|host: ||p')
llvm-cov show -Xdemangler=rustfilt \
  "target/$TARGET/coverage/$TARGET/release/$FUZZ_TARGET" \
  -instr-profile="fuzz/coverage/$FUZZ_TARGET/coverage.profdata"  \
  -show-line-counts-or-regions -show-instantiations  \
  -format=html -o $OUT $SRC_FILTER

xdg-open $OUT/index.html
