#!/bin/sh
# This script provides an easy way to test swiftlet during development
clear

if [ $# -eq 0 ]; then
    ../target/debug/swiftlet -n Client -a [::1]:0
else
    ../target/release/swiftlet -n Client -a [::1]:0
fi

exit 0
