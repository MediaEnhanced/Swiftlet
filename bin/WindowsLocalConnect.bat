cls
@echo off

rem Creates a server, waits 1 second, then creates a client to connect locally to the server
Start "Swiftlet Server" "Swiftlet.exe" --sname ServerName --ipv6
timeout /t 1
Start "Swiftlet Client" "Swiftlet.exe" -u LocalUserName -a [::1]:9001

echo.
exit 0
