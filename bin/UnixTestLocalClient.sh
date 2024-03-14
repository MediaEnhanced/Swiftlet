#!/bin/sh
# This script provides an easy way to test swiftlet during development
clear

if [ $# -eq 0 ]; then
    ../target/debug/swiftlet -u Client -a [::1]:9001
else
    ../target/release/swiftlet -u Client -a [::1]:9001
fi

exit 0
