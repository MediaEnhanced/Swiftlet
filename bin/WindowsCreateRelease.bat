cls
@echo off

rem Creates a Windows release but expects *BuildAndCopyHere* to be run first (and whenever the executable needs to be updated)
powershell Compress-Archive -Force -Path .\Swiftlet.exe, .\audio, .\security, .\WindowsLocalConnect.bat, .\WindowsRemoteConnect.bat -DestinationPath .\releases\SwiftletWindows.zip

echo.
exit 0
