@echo off
setlocal

rem Change only these values.
set "LINKDATA_A=D:\SteamLibrary\steamapps\common\OPPW4\LINKDATA\CMN\LINKDATA_A.BIN"
set "ENTRY_ID=247"
set "FORMAT=json"
set "OUT_FILE=moveset_%ENTRY_ID%.json"

rem Optional: add --typed-words if you want u32/i32/f32 annotations.
set "EXTRA_ARGS="

cd /d "%~dp0"
moveset-dump.exe "%LINKDATA_A%" %ENTRY_ID% --format %FORMAT% --out "%OUT_FILE%" %EXTRA_ARGS%

if errorlevel 1 (
  echo.
  echo Dump failed.
  pause
  exit /b 1
)

echo.
echo Dump written to "%~dp0%OUT_FILE%"
pause
