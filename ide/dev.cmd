@echo off
rem Abre el IDE de acm en modo desarrollo (recarga en caliente).
rem Compila antes el acm del repositorio: en desarrollo el IDE usa ..\target\release\acm.exe.
setlocal
cd /d "%~dp0"

echo [1/3] Compilando acm...
pushd ..
cargo build --release
if errorlevel 1 (popd & echo No se pudo compilar acm. & pause & exit /b 1)
popd

if not exist node_modules (
  echo [2/3] Instalando dependencias...
  call npm install
  if errorlevel 1 (echo Fallo npm install. & pause & exit /b 1)
) else (
  echo [2/3] Dependencias ya instaladas.
)

echo [3/3] Abriendo el IDE...
call npm run tauri dev
if errorlevel 1 pause
