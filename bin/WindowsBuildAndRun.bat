cls
@echo off

IF [%1] EQU [] (
	GOTO Release
) ELSE (
	GOTO Debug
)

:Debug
cargo.exe build
IF %ERRORLEVEL% NEQ 0 EXIT /b %ERRORLEVEL%
copy ..\target\debug\swiftlet.exe .\Swiftlet.exe
GOTO Run

:Release
cargo.exe build -r
IF %ERRORLEVEL% NEQ 0 EXIT /b %ERRORLEVEL%
strip.exe -o .\Swiftlet.exe ..\target\release\swiftlet.exe

:Run
Start "Networking Audio Server" ".\StartServer.bat"
timeout /t 1
Start "Networking Audio Client1" ".\StartClient.bat"
timeout /t 1
Start "Networking Audio Client2" ".\StartClient2.bat"
timeout /t 1
Start "Networking Audio Client3" ".\StartClient3.bat"

echo.
exit 0
