#!/usr/bin/env bash

DIR=$(dirname "$(readlink -f $BASH_SOURCE)")
cd $DIR/c || exit 1

make
./create-test-files

echo "Creating test file 3 (gresource file)"
glib-compile-resources test3.gresource.xml
mv test3.gresource ../data/
