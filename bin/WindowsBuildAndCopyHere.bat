cls
@echo off

rem Builds swiftlet in release mode and copies the executable to the this (bin) directory
cargo.exe build -r
IF %ERRORLEVEL% NEQ 0 EXIT /b %ERRORLEVEL%
copy ..\target\release\swiftlet.exe .\Swiftlet.exe

echo.
exit 0
