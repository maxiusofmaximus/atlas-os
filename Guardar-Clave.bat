@echo off
chcp 65001 >nul
setlocal
title Atlas OS - Guardar clave

rem --- Localiza el ejecutable atlas.exe ---
set "REPO=%~dp0"
set "EXE=%REPO%src-tauri\target\debug\atlas.exe"
if not exist "%EXE%" set "EXE=%REPO%src-tauri\target\release\atlas.exe"
if not exist "%EXE%" (
  echo.
  echo   No encuentro atlas.exe todavia.
  echo   Hay que compilarlo UNA vez. Abre una terminal en esta carpeta y ejecuta:
  echo.
  echo       cargo build --manifest-path src-tauri\Cargo.toml --bin atlas
  echo.
  pause
  exit /b 1
)

rem --- Archivo de claves (fuera del proyecto, en tu perfil) ---
set "DEST=%USERPROFILE%\.opencode\secretos.txt"
if not exist "%USERPROFILE%\.opencode" mkdir "%USERPROFILE%\.opencode"
if not exist "%DEST%" copy /Y "%REPO%secretos.plantilla.txt" "%DEST%" >nul

echo.
echo   Se abrira el Bloc de notas con tus claves.
echo   Escribe la clave tras el signo "=", guarda y CIERRA el Bloc de notas.
echo.
start /wait notepad "%DEST%"

echo.
echo   Guardando en el llavero seguro de Windows...
"%EXE%" secrets import "%DEST%" --consume

echo.
echo   Listo. Puedes cerrar esta ventana.
pause
