# ADR 0002: El Excel se genera en Rust

**Fecha:** 2026-10-15 · **Estado:** vigente · Decisión D7 del [spec](../superpowers/specs/2026-10-15-qapture-design.md).

## Contexto

Un proyecto real puede tener cientos de capturas. Generar el `.xlsx` en el WebView obligaría a mover esas imágenes al front y cargar el proceso de la interfaz.

## Decisión

El archivo se genera en el núcleo Rust con `rust_xlsxwriter`: carátula, matriz, evidencias con imágenes incrustadas y resumen con gráficos nativos.

## Consecuencias

- Exportar es robusto con muchas imágenes y no bloquea la interfaz.
- Los tests de Rust pueden abrir el `.xlsx` generado con `calamine` y verificar hojas, celdas, hipervínculos y número de imágenes.
- El resultado hay que revisarlo a mano en Excel y WPS en cada release ([checklist](../release-checklist.md)).
- Cambiar el formato exige tocar Rust, no solo el front.
