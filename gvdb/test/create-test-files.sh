#!/usr/bin/env bash

git submodule init || exit 1
git submodule update || exit 1

DIR=$(dirname "$(readlink -f $BASH_SOURCE)")
cd $DIR/c/create-test-files || exit 1

make
./create-test-files

cd $DIR/data/gresource || exit 1
echo "Creating test file 3 (gresource file)"
glib-compile-resources test3.gresource.xml
mv test3.gresource ../
