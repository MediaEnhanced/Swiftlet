cls
@echo off

rem This script provides an easy way to test swiftlet during development

IF [%1] EQU [] (
	GOTO Debug
) ELSE (
	GOTO Release
)

:Debug
cargo.exe build-windows-x64
IF %ERRORLEVEL% NEQ 0 EXIT /b %ERRORLEVEL%

Start "Swiftlet Server" "..\target\x86_64-pc-windows-gnu\debug\swiftlet.exe" --sname Server --ipv6
timeout /t 1
Start "Swiftlet Client1" "..\target\x86_64-pc-windows-gnu\debug\swiftlet.exe" -u Client1 -a [::1]:9001
rem timeout /t 1
rem Start "Swiftlet Client2" "..\target\x86_64-pc-windows-gnu\debug\swiftlet.exe" -u Client2 -a [::1]:9001
rem timeout /t 1
rem Start "Swiftlet Client3" "..\target\x86_64-pc-windows-gnu\debug\swiftlet.exe" -u Client3 -a [::1]:9001

echo.
exit 0



:Release
cargo.exe build-windows-x64 -r
IF %ERRORLEVEL% NEQ 0 EXIT /b %ERRORLEVEL%

Start "Swiftlet Server" "..\target\x86_64-pc-windows-gnu\release\swiftlet.exe" --sname Server --ipv6
timeout /t 1
Start "Swiftlet Client1" "..\target\x86_64-pc-windows-gnu\release\swiftlet.exe" -u Client1 -a [::1]:9001
timeout /t 1
Start "Swiftlet Client2" "..\target\x86_64-pc-windows-gnu\release\swiftlet.exe" -u Client2 -a [::1]:9001
timeout /t 1
Start "Swiftlet Client3" "..\target\x86_64-pc-windows-gnu\release\swiftlet.exe" -u Client3 -a [::1]:9001

echo.
exit 0
