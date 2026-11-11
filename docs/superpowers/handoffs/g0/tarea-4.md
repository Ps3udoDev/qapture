# Tarea 4 · Verificación manual y go / no-go

**Estado:** DONE. Veredicto **go**. El detalle está en [pico-trasplante.md](pico-trasplante.md).

## Qué se agregó
- `src-tauri/src/excel/origin/evidence_graft.rs`: test ignorado `write_files_for_manual_check` (`#[ignore = "genera archivos para abrir a mano"]`) dentro del módulo `tests`. Lee `QAPTURE_SPIKE_OUT` (si falta o no es una carpeta, falla con `Define QAPTURE_SPIKE_OUT con una carpeta existente.`) y escribe:
  - `guion_sintetico.xlsx` con `guion_fixture::build()`.
  - `guion_con_evidencias.xlsx` con `graft_sheet(&guion_fixture::build(), &donor(3), "Evidencias")`.
- Ajuste de formato: `cargo fmt` reformateó la línea del `assert!` del test.

## Comandos y salidas (desde `src-tauri/`)
- `cargo test write_files_for_manual_check -- --ignored`: `test excel::origin::evidence_graft::tests::write_files_for_manual_check ... ok`, `1 passed; 0 failed; 0 ignored`. La advertencia de `linker_messages` sobre `qapture_lib.dll.lib` es del enlazador y no afecta el resultado.
- Archivos generados:
  - `C:\Users\DELL\AppData\Local\Temp\g0\guion_sintetico.xlsx` (10698 bytes)
  - `C:\Users\DELL\AppData\Local\Temp\g0\guion_con_evidencias.xlsx` (13320 bytes)
- `cargo fmt --check`: sin diferencias tras `cargo fmt` (exit 0).
- `cargo clippy --all-targets -- -D warnings`: sin errores (exit 0).
- `cargo test excel::origin`: `12 passed; 0 failed; 1 ignored` (el ignorado es `write_files_for_manual_check`).

## Verificación manual (Paso 3)
Una persona la hizo en Excel de escritorio y en LibreOffice Calc. No salió ningún aviso, y el guardado desde Excel se volvió a abrir sin problemas. La tabla está en [pico-trasplante.md](pico-trasplante.md).

## Commit
Solo el `.rs` y este handoff. Los `.xlsx` generados están fuera del repo (en `%TEMP%\g0`) y no se incluyen.
