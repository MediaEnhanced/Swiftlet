cls
@echo off

rem Creates a server, waits 1 second, then creates a client to connect locally to the server
Start "Swiftlet Server" "Swiftlet.exe" -n ServerName
timeout /t 1
Start "Swiftlet Client" "Swiftlet.exe" -n LocalUserName -a [::1]:0

echo.
exit 0
