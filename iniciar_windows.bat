@echo off
:: Arrancar Imagen Duplicada en Windows
cd /d "%~dp0"

if not exist "node_modules" (
  echo Instalando dependencias...
  call npm install
)

echo Iniciando Imagen Duplicada...
call npm run tauri dev