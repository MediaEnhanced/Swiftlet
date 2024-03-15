#!/bin/sh
# Builds swiftlet in release mode and copies the executable to the this (bin) directory

if [ $# -eq 0 ]; then
    cargo build -r
else
    cargo build -r --no-default-features
fi
if [ $? -ne 0 ]; then
    exit $?
fi
cp ../target/release/swiftlet ./swiftlet

exit 0
