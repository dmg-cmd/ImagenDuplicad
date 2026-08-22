# Imágenes de ejemplo

Para probar la detección de duplicados y el comparador de la app.

## Archivos

| Archivo | Descripción |
|---|---|
| `a_original.png` | Imagen base |
| `b_recodificada.png` | La base re-guardada (casi idéntica, agrupa como "probable") |
| `f_marca_dmg.png` | La base con marca de agua **DMG** |
| `i_marca_0af.png` | La base con marca de agua **0AF** |
| `h_mas_brillante.png` | La base con brillo +8% (diferencia global sutil) |
| `g_recortada.png` | La base recortada 5% por lado (no se agrupa: cambian las dimensiones) |
| `c_unico.png`, `d_otro_unico.png` | Imágenes sin duplicados |

## Cómo probar

1. Escanea esta carpeta con la app: `recodificada`, `marca_dmg` y `mas_brillante`
   salen juntas como grupo **probable**.
2. Pulsa **"Comparar de a dos"** seleccionando dos imágenes del grupo:
   - **Cortina**: arrastra el divisor por la esquina inferior derecha para
     descubrir cada marca de agua.
   - **Fundido**: atenúa una sobre otra hasta verlas mezcladas.
   - **Diferencia**: compara `f_marca_dmg.png` vs `i_marca_0af.png` — ambos
     textos aparecen resaltados en rojo.
   - **Doble clic** sobre cualquier zona: centra ambas imágenes ahí con zoom alto.
