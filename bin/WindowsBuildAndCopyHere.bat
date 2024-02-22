cls
@echo off

rem Builds swiftlet in release mode and copies the executable to the this (bin) directory
cargo.exe build-windows-x64 -r
IF %ERRORLEVEL% NEQ 0 EXIT /b %ERRORLEVEL%
copy ..\target\x86_64-pc-windows-gnu\release\swiftlet.exe .\Swiftlet.exe

echo.
exit 0
