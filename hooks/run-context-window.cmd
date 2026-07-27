@echo off
setlocal

if not defined PLUGIN_ROOT set "PLUGIN_ROOT=%~dp0.."

set "architecture=%PROCESSOR_ARCHITECTURE%"
if defined PROCESSOR_ARCHITEW6432 set "architecture=%PROCESSOR_ARCHITEW6432%"

if /I "%architecture%"=="AMD64" set "target=x86_64-pc-windows-msvc"
if /I "%architecture%"=="ARM64" set "target=aarch64-pc-windows-msvc"

if not defined target (
  echo codex-context-window: unsupported Windows architecture: %architecture% 1>&2
  exit /b 1
)

set "binary=%PLUGIN_ROOT%\bin\%target%\codex-context-window.exe"
if exist "%binary%" goto run

set "binary=%PLUGIN_ROOT%\target\%target%\release\codex-context-window.exe"
if exist "%binary%" goto run

set "binary=%PLUGIN_ROOT%\target\release\codex-context-window.exe"
if exist "%binary%" goto run

echo codex-context-window: native binary is missing for %target% 1>&2
exit /b 1

:run
"%binary%"
set "status=%ERRORLEVEL%"
endlocal & exit /b %status%
