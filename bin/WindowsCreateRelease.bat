cls
@echo off

rem Creates a Windows release of Swiftlet (for host architecture)
cargo.exe install --path ../
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%
powershell Compress-Archive -Force -Path .\swiftlet.exe, .\audio, .\security, .\WindowsLocalConnect.bat, .\WindowsRemoteConnect.bat -DestinationPath .\releases\SwiftletWindows.zip

echo.
exit 0
