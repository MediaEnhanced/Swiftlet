@echo off
setlocal EnableDelayedExpansion

set params=%*
echo Initial parameters: !params!

set firstChar=!params:~0,1!
if !firstChar!==@ (
    rem echo Found File: !params!

    set paramFile=!params:@=!
    rem echo Original File Parameters:
    for /f "delims=" %%l in ('type "!paramFile!" ^& break ^> "!paramFile!" ') do (
        rem echo %%l
        set "line=%%l "

        if "!line:libcompiler_builtins=!"=="!line!" (
            set line=!line:-Wl,--disable-auto-image-base=!
            set line=!line:-Wl,-Bdynamic=-Wl,-search_paths_first!
            set line=!line:-lgcc_eh=!
            set line=!line:-lmsvcrt=!
						set line=!line:-lgcc_s=!
            set line=!line:-lgcc=!
            set line=!line:-l:libpthread.a=!
            >>"!paramFile!" echo(!line!
        )
    )

    rem echo Modified File Parameters:
    rem for /f "delims=" %%l in (!paramFile!) do (
    rem     echo %%l
    rem )
)

set params=!params:-Wl,--disable-auto-image-base=!
set params=!params:-Wl,-Bdynamic=-Wl,-search_paths_first!
set params=!params:-lgcc_eh=!
set params=!params:-lmsvcrt=!
set params=!params:-lgcc_s=!
set params=!params:-lgcc=!
set params=!params:-l:libpthread.a=!

set builtinNum=!params:*libcompiler_builtins-=!
set builtinNum=%builtinNum:.rlib=&rem.%
set "builtin=libcompiler_builtins-%builtinNum%.rlib"

set winTest=!params:-lkernel32=!
if not !winTest!==!params! (
    set params=!params:%builtin%=!
)

set "params=!params! -lunwind"
rem set params=!params:-lmingw32=!
rem set params=!params:-lmingwex=!

echo Adjusted parameters: !params!

zig.exe cc !params!
