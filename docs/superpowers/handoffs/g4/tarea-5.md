# Tarea 5 · Comandos y contrato (G4-T5)

**Estado:** DONE
**Commits:** `78277f4` (`feat(g4): comandos export_origin y export_origin_info con su contrato`)

## Qué se implementó

1. `src-tauri/src/dto.rs`:
   - `OriginInfoDto`: `{ file_name, sheet, cases_written, unmapped, evidence, formulas, unavailable, suggested_name }`.
   - `OriginExportInput`: `{ tester, observations, include_evidence, image_max_width }` con `deny_unknown_fields`.
   - Añadidos a `sample()`, `export_samples()`, `inputs_json()`, `inputs_deserialize_from_ts_shape()` y `inputs_reject_unknown_fields()`.

2. `src-tauri/src/commands/origin.rs`:
   - `suggested_name(file_name)`: genera `<stem>_ejecutado.xlsx` (D11).
   - `validate_origin_export_input(input)`: valida `tester` obligatorio (máx 100 caracteres), `observations` (máx 500 caracteres, mensaje `"La observación supera 500 caracteres."`) y `image_max_width` dentro de `IMAGE_WIDTH_RANGE` (400..=1600 px).
   - `info(conn, files, project_id, tester)`: función pura y testeable que comprueba disponibilidad (`check_copy`), calcula conteos de casos escritos, casos no mapeados, total de evidencias y fórmulas en riesgo (`formulas_at_risk`), o devuelve `unavailable` si la copia falta o cambió.
   - Comando `export_origin_info`: extrae el tester configurado en el perfil de Ajustes y llama a `info`.
   - Comando `export_origin`: valida el input, lee `OriginData` y comprueba disponibilidad antes de abrir el diálogo, abre el diálogo «Guardar como» nativo mediante `ask_destination`, verifica permisos de escritura con `save::ensure_writable`, emite eventos de progreso `EXPORT_PROGRESS_EVENT`, ejecuta `origin::export::build`, guarda atómicamente con `save::save_bytes_atomic`, registra la ruta en `state.exports().remember(&dest)` y devuelve `ExportResultDto`.
   - Tests unitarios en `commands::origin::tests`:
     - `suggested_name_adds_ejecutado`
     - `validate_input_accepts_valid_data`
     - `validate_input_rejects_empty_tester`
     - `validate_input_rejects_long_tester`
     - `validate_input_rejects_observations_over_500_chars`
     - `validate_input_rejects_image_width_out_of_range`
     - `info_is_none_for_projects_not_imported`
     - `info_reports_unavailable_copy`
     - `info_counts_cases_evidence_formulas_and_unmapped`
     - `info_detects_formulas_at_risk`

3. Registro de comandos y permisos:
   - `src-tauri/src/commands/mod.rs`: expone `pub mod origin;`.
   - `src-tauri/src/commands/names.rs`: añade `"export_origin_info"` y `"export_origin"` a `APP_COMMANDS`.
   - `src-tauri/src/commands/export.rs`: expone `pub(crate) fn ask_destination`.
   - `src-tauri/src/excel/save.rs`: elimina `#[allow(dead_code)]` de `save_bytes_atomic`.
   - `src-tauri/capabilities/main.json`: añade permisos `"allow-export-origin-info"` y `"allow-export-origin"`.
   - `src-tauri/src/lib.rs`: registra los dos comandos en `tauri::generate_handler!`.

4. Contrato TypeScript:
   - Fixture `src/infrastructure/tauri/__fixtures__/contracts.json` regenerado con `UPDATE_CONTRACTS=1 cargo test contract`.
   - `src/infrastructure/tauri/dto.ts`: esquemas Zod `originInfoSchema` y `originExportInputSchema`, y tipos inferidos `OriginInfoDto` y `OriginExportInputDto`.
   - `src/infrastructure/tauri/contract.test.ts`: tests de contrato Rust → TS (`originInfo`) y TS → Rust (`originExportInput`).

## Archivos
- `src-tauri/src/commands/origin.rs` (nuevo)
- `src-tauri/src/commands/mod.rs` (modificado)
- `src-tauri/src/commands/names.rs` (modificado)
- `src-tauri/src/commands/export.rs` (modificado)
- `src-tauri/src/excel/save.rs` (modificado)
- `src-tauri/src/lib.rs` (modificado)
- `src-tauri/capabilities/main.json` (modificado)
- `src-tauri/src/dto.rs` (modificado)
- `src/infrastructure/tauri/dto.ts` (modificado)
- `src/infrastructure/tauri/contract.test.ts` (modificado)
- `src/infrastructure/tauri/__fixtures__/contracts.json` (modificado)

## Pruebas
- RED:
  - `cargo test commands::origin` falló inicialmente antes de la corrección de fixtures en tests.
  - `commands::names` falló hasta registrar los comandos en las tres listas (names, capabilities, handlers).
- GREEN:
  - `cargo test commands::origin` → 10 passed; 0 failed.
  - `cargo test names` → 15 passed; 0 failed (incluye `every_registered_handler_is_listed_and_vice_versa` y `main_window_gets_core_and_every_non_overlay_command`).
  - `cargo test contract` → 3 passed; 0 failed (`dto_contract_matches_fixture`).
  - `cargo test` (suite completa en `src-tauri`) → 608 passed; 0 failed; 4 ignored.
  - `npx vitest run src/infrastructure/tauri/contract.test.ts` → 45 passed (45).
  - `npx vitest run` (suite completa de frontend) → 61 test files passed, 428 passed.
- Lint y tipos:
  - `cargo fmt --check` → limpio (código 0).
  - `cargo clippy --all-targets -- -D warnings` → limpio (código 0).
  - `npm run lint` → limpio (código 0, 0 warnings).
  - `npm run typecheck` → limpio (código 0).

## Autorrevisión
- Los nombres sugeridos respetan la regla D11 (`<nombre_sin_xlsx>_ejecutado.xlsx`).
- La validación rechaza observaciones de más de 500 caracteres con el mensaje exacto exigido en el plan.
- Los comandos sueltan la conexión a SQLite antes de abrir el diálogo interactivo «Guardar como».
- El contrato de serialización entre Rust y TypeScript está perfectamente sincronizado y verificado mediante Zod y Vitest.

## Dudas o riesgos
- Ninguno. Listo para implementar la Task 6 (UI y puertos de frontend para la opción de formato de origen).

## Revisión (la llena el controlador)
- Spec: Pendiente · Calidad: Pendiente
- Hallazgos y cómo se resolvieron
