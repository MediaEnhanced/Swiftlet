cls
@echo off

cargo.exe doc --no-deps
IF %ERRORLEVEL% NEQ 0 EXIT /b %ERRORLEVEL%
Robocopy.exe ..\target\doc\ ..\docs

echo.
exit 0
