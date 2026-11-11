# Pico · Trasplante de la hoja «Evidencias» (D10)

**Veredicto:** ✅ **go**. G4-T3 parte de `graft_sheet`.
**Fecha:** 2026-11-10 · **Rama:** `feat/g0-fixture-y-pico`

## Qué se construyó
- **Tarea 1** (`d032ece`): `excel/guion_fixture.rs` genera en memoria un guion sintético con la forma del guion de referencia: carátula en las filas 1–12, encabezados en la fila 15, 36 casos, celdas combinadas, listas en H16:H35 y K16:K35 (con «Crìtico»), «Hoja1» oculta, autofiltro `A15:M15`, área de impresión `A1:M51` y logo. No lleva datos reales.
- **Tarea 2** (`f1febf0`, `d5f808f`): `excel/origin/parts.rs` lee un `.xlsx` parte por parte y lo reescribe sin alterarlo: mismos bytes y mismo método de compresión en cada parte.
- **Tarea 3** (`93a62c3`): `excel/origin/evidence_graft.rs`, con `graft_sheet(libro, donante, nombre)`. Injerta en el libro del cliente una hoja generada con `rust_xlsxwriter` (texto e imágenes). Solo cambian las tres partes índice (`xl/workbook.xml`, `xl/_rels/workbook.xml.rels` y `[Content_Types].xml`); el resto queda igual byte a byte. No pisa `sheetN`, `drawingN` ni `imageN` existentes.
- **Tarea 4** (`6f0c962`): test ignorado `write_files_for_manual_check`, que escribe `guion_sintetico.xlsx` y `guion_con_evidencias.xlsx` (`graft_sheet(&build(), &donor(3), "Evidencias")`) para revisarlos a mano.

## Verificación manual
Una persona abrió `guion_con_evidencias.xlsx` en Excel de escritorio y en LibreOffice Calc. Dio un solo resultado para los dos programas.

| Comprobación | Esperado | Excel | LibreOffice |
|---|---|---|---|
| Aviso de reparación al abrir | Ninguno | ✅ Ninguno | ✅ Ninguno |
| Pestañas | «Guión de Pruebas», «Evidencias» (Hoja1 sigue oculta) | ✅ | ✅ |
| Hoja «Evidencias» | Los dos textos y las 3 imágenes visibles | ✅ A1 «Evidencias de pruebas», A2 «CP05 — Adjuntar archivo no Excel» y 3 imágenes | ✅ Igual |
| Hoja del guion | Logo, celdas combinadas, autofiltro en la fila 15, listas en K16:K35 y H16:H35 | ✅ | ✅ |
| Área de impresión | `A1:M51` en la vista preliminar | ✅ | ✅ |
| Guardar desde Excel y volver a abrir | Sin aviso | ✅ Sin aviso; todo se conserva | — (solo Excel) |

## Limitaciones conocidas (para G4-T3)
- La hoja injertada no lleva estilos: ni negritas ni rellenos. Es a propósito: no se tocan `xl/styles.xml` ni los estilos del libro del cliente. Si G4 necesita formato, tendrá que agregar estilos al `styles.xml` del cliente sin cambiar los índices que ya existen.
- `docProps/app.xml` no lista la hoja nueva. Ni Excel ni LibreOffice se quejaron. Excel lo corrige solo al guardar.
- Los 6 Minor de la revisión de la tarea 3 (endurecimiento) quedan anotados en [tarea-3.md](tarea-3.md) para G4-T3.
