cls
@echo off

IF [%1] EQU [] (
	GOTO Debug
) ELSE (
	GOTO Release
)

:Debug
cargo.exe build
IF %ERRORLEVEL% NEQ 0 EXIT /b %ERRORLEVEL%

Start "Networking Audio Server" "..\target\debug\swiftlet.exe" --name Realm
timeout /t 1
Start "Networking Audio Client1" "..\target\debug\swiftlet.exe" -n Jared ::1
timeout /t 1
Start "Networking Audio Client2" "..\target\debug\swiftlet.exe" -n Client2 ::1
timeout /t 1
Start "Networking Audio Client3" "..\target\debug\swiftlet.exe" -n Client3 ::1

echo.
exit 0



:Release
cargo.exe build -r
IF %ERRORLEVEL% NEQ 0 EXIT /b %ERRORLEVEL%

rem stripping is now part of the cargo build process
rem strip.exe -o ..\target\release\swiftlet.exe ..\target\release\swiftlet.exe

Start "Networking Audio Server" "..\target\release\swiftlet.exe" --name Realm
timeout /t 1
Start "Networking Audio Client1" "..\target\release\swiftlet.exe" -n Jared ::1
timeout /t 1
Start "Networking Audio Client2" "..\target\release\swiftlet.exe" -n Client2 ::1
timeout /t 1
Start "Networking Audio Client3" "..\target\release\swiftlet.exe" -n Client3 ::1

echo.
exit 0
