cls
@echo off

rem Builds swiftlet in release mode and copies the executable to the this (bin) directory
cargo.exe build -r
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%
copy ..\target\release\swiftlet.exe .\swiftlet.exe

echo.
exit 0
