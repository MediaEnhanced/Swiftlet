cls
@echo off

rem Modify the IPv6 address in the [] brackets to connect to a remote Swiftlet Server
Start "Swiftlet Client" "Swiftlet.exe" -u RemoteUserName -a [2606:4700:4700::1111]:9001

echo.
exit 0
