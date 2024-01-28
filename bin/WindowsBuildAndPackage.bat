cls
@echo off

cargo.exe build -r
IF %ERRORLEVEL% NEQ 0 EXIT /b %ERRORLEVEL%
copy ..\target\release\swiftlet.exe .\Swiftlet.exe
powershell Compress-Archive -Force -Path .\Swiftlet.exe, .\audio, .\security -DestinationPath .\Swiftlet.zip

echo.
exit 0
