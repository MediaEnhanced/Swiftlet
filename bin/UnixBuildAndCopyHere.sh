#!/bin/sh
# Builds swiftlet in release mode and copies the executable to the this (bin) directory
cargo build -r
if [ $? -ne 0 ]; then
    exit $?
fi
cp ../target/release/swiftlet ./swiftlet

exit 0
