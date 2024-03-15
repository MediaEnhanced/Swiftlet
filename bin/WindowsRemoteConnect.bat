cls
@echo off

rem Modify the IPv6 address in the [] brackets to connect to a remote Swiftlet Server
Start "Swiftlet Client" "Swiftlet.exe" -u RemoteUserName -a [2604:a880:4:1d0::6e8:4000]:9001

echo.
exit 0
