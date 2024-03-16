cls
@echo off

rem This script provides an easy way to test swiftlet during development

if [%1] EQU [] (
	goto Debug
) else (
	goto Release
)

:Debug
cargo.exe build
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

Start "Swiftlet Server" "..\target\debug\swiftlet.exe" -n Server
timeout /t 1
Start "Swiftlet Client1" "..\target\debug\swiftlet.exe" -n Client1 -a [::1]:0
timeout /t 1
Start "Swiftlet Client2" "..\target\debug\swiftlet.exe" -n Client2 -a [::1]:0
timeout /t 1
Start "Swiftlet Client3" "..\target\debug\swiftlet.exe" -n Client3 -a [::1]:0

echo.
exit 0



:Release
cargo.exe build -r
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

Start "Swiftlet Server" "..\target\release\swiftlet.exe" -n Server
timeout /t 1
Start "Swiftlet Client1" "..\target\release\swiftlet.exe" -n Client1 -a [::1]:0
timeout /t 1
Start "Swiftlet Client2" "..\target\release\swiftlet.exe" -n Client2 -a [::1]:0
timeout /t 1
Start "Swiftlet Client3" "..\target\release\swiftlet.exe" -n Client3 -a [::1]:0

echo.
exit 0
