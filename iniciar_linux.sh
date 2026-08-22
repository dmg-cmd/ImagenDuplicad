#!/bin/bash
# Arrancar Imagen Duplicada en Linux
cd "$(dirname "$0")"

# Cargar Rust/Cargo en PATH
source "$HOME/.cargo/env" 2>/dev/null

if [ ! -d "node_modules" ]; then
  echo "Instalando dependencias..."
  npm install
fi

echo "Iniciando Imagen Duplicada..."
npm run tauri dev

echo ""
echo "Presiona Enter para cerrar..."
read