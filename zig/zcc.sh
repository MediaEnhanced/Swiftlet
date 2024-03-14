#!/bin/sh
# Uses zig (cc) as a gcc replacement for cross compiling/linking

params=$@
#echo Initial parameters: $params

# Might need to add file parameter replacement if that ends up being
# a unix cross compile error in the future

params=${params//-Wl,--disable-auto-image-base/}
params=${params//-Wl,-Bdynamic/-Wl,-search_paths_first}
params=${params//-lgcc_eh/}
params=${params//-lmsvcrt/}
params=${params//-lgcc_s/}
params=${params//-lgcc/}
params=${params//-l:libpthread.a/}

# Replace libcompiler_builtins-*rlib with libstd*rlib
libstdPre=${params#*libstd-}
libstdMid=${libstdPre/rlib*/}
libstd="libstd-${libstdMid}rlib"
params=${params//libcompiler_builtins*rlib/$libstd}

params+=" -lunwind"

#echo Adjusted Parameters: $params
zig cc $params
