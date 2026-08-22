#!/bin/bash
# grabar_presentacion.sh
# Graba la presentacion HTML como video usando ffmpeg + x11grab
# Uso: ./grabar_presentacion.sh [duracion_por_slide_segundos] [resolucion]
#
# Requisitos: ffmpeg, xdotool, navegador web

DURATION=${1:-8}
RESOLUTION=${2:-1280x720}
OUTPUT="presentacion_video.mp4"
HTML_PATH="$(dirname "$0")/presentacion.html"

echo "=== Grabador de presentacion ==="
echo "Duracion por slide: ${DURATION}s"
echo "Resolucion: ${RESOLUTION}"
echo "Output: ${OUTPUT}"
echo ""

# Abrir la presentacion en el navegador
echo "Abriendo presentacion en navegador..."
xdg-open "file://$(realpath "$HTML_PATH")" 2>/dev/null || xdg-open "$(realpath "$HTML_PATH")"
sleep 3

echo "Iniciando grabacion..."
echo "Presiona Ctrl+C para detener."

ffmpeg -f x11grab -framerate 30 -video_size "$RESOLUTION" -i :0.0 \
  -c:v libx264 -preset medium -crf 23 \
  -pix_fmt yuv420p \
  "$OUTPUT" 2>&1 | tail -5 &

FFPID=$!
sleep 2

# Avanzar slides cada N segundos
for i in $(seq 1 13); do
  echo "Slide $i/13..."
  sleep "$DURATION"
  xdotool key Right 2>/dev/null || true
done

sleep 2
kill $FFPID 2>/dev/null
wait $FFPID 2>/dev/null

echo ""
echo "Video generado: $OUTPUT"
echo "Para subirlo a Gemini Notebook, arrastra el archivo .mp4"