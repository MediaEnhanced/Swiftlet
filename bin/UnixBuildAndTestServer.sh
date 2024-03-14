#!/bin/sh
# This script provides an easy way to test swiftlet during development
clear

if [ $# -eq 0 ]; then
    cargo build
    if [ $? -ne 0 ]; then
        exit $?
    fi

    ../target/debug/swiftlet --sname Server --ipv6
else
    cargo build -r
    if [ $? -ne 0 ]; then
        exit $?
    fi

    ../target/release/swiftlet --sname Server --ipv6
fi

exit 0
