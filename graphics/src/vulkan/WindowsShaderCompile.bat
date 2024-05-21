cls
@echo off

rem This script provides an easy way to test compile the glsl shaders

if [%1] EQU [] (
	goto Fragment
) else (
	goto Vertex
)

:Vertex
glslang.exe --target-env vulkan1.3 -o ..\..\..\bin\triangle-font-vert.spv triangle-font.vert.glsl
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

:Fragment
glslang.exe --target-env vulkan1.3 -o ..\..\..\bin\triangle-font-frag.spv triangle-font.frag.glsl
if %ERRORLEVEL% NEQ 0 exit /b %ERRORLEVEL%

echo.
exit 0
