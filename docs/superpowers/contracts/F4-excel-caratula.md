# F4 — Excel con carátula Teams4Soft · `v0.4.0`

**Depende de:** F1 (y F2 para incluir imágenes) · **Plan:** [2026-10-16-f4-excel-caratula.md](../plans/2026-10-16-f4-excel-caratula.md)

## Objetivo
Exportar el proyecto a un `.xlsx` con las hojas **Carátula**, **Matriz** (columnas dinámicas), **Evidencias** y **Resumen**, con branding Teams4Soft, generado en Rust con `rust_xlsxwriter`.

## Alcance
| Entra | Fuera |
|---|---|
| UC-05 Control de versiones del documento (tabla en el proyecto) | Plantillas de Excel personalizadas por cliente |
| UC-16 Exportar con diálogo (tester, versión, hojas) y "Guardar como" nativo | Exportar a PDF |
| UC-17 Exportar plantilla vacía con las columnas del proyecto | |
| Nombre sugerido `T4S_<Cliente>_<Proyecto>_Evidencias_<AAAAMMDD>_v<versión>.xlsx` | |
| Nombre del tester recordado (preferencia local hasta que F6 tenga Settings) | |

## Tasks
| ID | Task |
|---|---|
| F4-T1 | Migración `0003_doc_version.sql`, repositorio y comandos de versiones del documento + UI en Configuración del proyecto |
| F4-T2 | `excel/styles.rs`: formatos T4S (Arial, `#2B629E`, `#BAC82D`, `#292B2D`, `#F6F4F1`) |
| F4-T3 | `excel/cover.rs`: banner, logo (`include_bytes!`), datos del proyecto, tester, KPI, tabla de versiones, pie |
| F4-T4 | `excel/matrix.rs`: columnas visibles en orden y ancho, tipos (fecha `dd/mm/yyyy`, número, sí/no), validación de lista y estado, formato condicional, autofiltro, paneles inmovilizados, impresión A4 horizontal |
| F4-T5 | `excel/evidence_sheet.rs`: bloques por caso, vínculos ida y vuelta, imágenes renderizadas con ancho máximo configurable |
| F4-T6 | `excel/summary.rs`: tablas por estado y por módulo/entorno + gráfico de barras nativo |
| F4-T7 | Comando `export_project(projectId, options)` con progreso (`export:progress`), escritura atómica y manejo de "archivo abierto en Excel" |
| F4-T8 | UI: diálogo de exportación, toast con "Abrir archivo" y "Abrir carpeta" |
| F4-T9 | Tests: leer el xlsx con `calamine` y verificar hojas, celdas, vínculos y número de imágenes |

## Notas de implementación (acordadas en el plan)
- crates.io retiró (*yank*) toda la serie `zip` < 9.0.3. Por eso `rust_xlsxwriter` se toma de su repositorio (commit `ade0a01`, ya con zip 9) hasta que publique una versión con zip 9. `calamine` (solo tests) usa `zip` 8.6.0 desde el tag del repositorio mediante `[patch.crates-io]`.
- Plantilla vacía: Carátula (sin KPI) + Matriz con 100 filas con formato y listas. Sin Evidencias ni Resumen, y sin versión en el nombre.
- Ancho máximo de imagen: 940 px por defecto (rango 400–1600). Las capturas más anchas se reducen al exportar. F6 lo lleva a Settings.
- «Abrir archivo» y «Abrir carpeta» solo aceptan archivos exportados en la sesión.
- El tester se recuerda en el equipo (`localStorage`) hasta F6-T9.

## Interfaces producidas
- `export_project` y `export_template`. F6 agrega "carpeta por defecto" y "ancho máximo de imagen" en Settings.

## Criterios de aceptación
- [ ] El xlsx generado se relee con `calamine`: 4 hojas, encabezados = columnas visibles, N filas = N casos, vínculos válidos.
- [ ] 300 evidencias se exportan en menos de 20 s y con memoria estable.

## Checklist de pruebas locales
| # | Prueba | Esperado | OK |
|---|---|---|---|
| 1 | Exportar proyecto con 10 casos y 20 evidencias | Abre en **Excel** sin advertencias | ☐ |
| 2 | El mismo archivo en **WPS** | Abre; logo, colores y vínculos funcionan | ☐ |
| 3 | Carátula | Logo T4S, tester, cliente, fechas, KPI correctos, tabla de versiones | ☐ |
| 4 | Matriz | Columnas personalizadas en su orden; estados con color; filtro activo | ☐ |
| 5 | "Ver evidencias" → "Volver a la matriz" | Navega correctamente | ☐ |
| 6 | Exportar con el archivo abierto en Excel | Mensaje claro para cerrarlo | ☐ |
| 7 | Imprimir la Matriz | A4 horizontal, ajustada al ancho, encabezados repetidos | ☐ |
| 8 | Plantilla vacía | Columnas del proyecto, sin datos, con carátula | ☐ |
| 9 | Exportar un proyecto con ~300 evidencias con el Administrador de tareas abierto | Termina en menos de 20 s; la memoria de Qapture vuelve a su nivel al terminar | ☐ |
