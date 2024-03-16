#!/bin/sh
# Connect remotely to a Swiftlet server specified by the IPv6 address in the [] brackets
clear
./swiftlet -n ClientRemote -a [2606:4700:4700::1111]:0
exit 0
