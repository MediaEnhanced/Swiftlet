#!/bin/sh
# Builds/Installs swiftlet in release mode and copies the executable to the this (bin) directory
cargo install --path ../
if [ $? -ne 0 ]; then
    exit $?
fi
mkdir ./releases
zip -r ./releases/SwiftletUnix.zip ./swiftlet ./audio/ ./security/

exit 0
