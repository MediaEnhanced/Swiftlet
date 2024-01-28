cls
@echo off

cargo.exe build -r
IF %ERRORLEVEL% NEQ 0 EXIT /b %ERRORLEVEL%
copy ..\target\release\swiftlet.exe .\Swiftlet.exe
rem Adding Zipping Relevant Files Here in Future

echo.
exit 0
