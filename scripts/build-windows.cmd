@echo off
rem Build openOMSI for Windows x64 into dist\windows (openomsi.exe is the game and, started
rem with no arguments, the launcher window). Needs Rust x86_64 MSVC (https://rustup.rs) and
rem Visual Studio Build Tools with "Desktop development with C++" and the Windows SDK.
rem To build the Windows version on a Mac, use scripts/build-windows-cross.sh instead.
setlocal
cd /d "%~dp0\.."
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo >nul 2>nul
if errorlevel 1 (
  echo Install Rust from https://rustup.rs using the MSVC toolchain, then run this script again.
  exit /b 1
)
cargo build --locked --release --target x86_64-pc-windows-msvc -p omsi-app -p omsi-launcher-core
if errorlevel 1 goto :failed
if not exist "dist\windows" mkdir "dist\windows"
copy /y "target\x86_64-pc-windows-msvc\release\openomsi.exe" "dist\windows\openomsi.exe" >nul || goto :failed
copy /y "target\x86_64-pc-windows-msvc\release\openomsi-launcher.exe" "dist\windows\openomsi-launcher.exe" >nul || goto :failed
echo.
echo Done. Run: "%CD%\dist\windows\openomsi.exe"
exit /b 0
:failed
echo.
echo Build failed. See the error above.
exit /b 1
