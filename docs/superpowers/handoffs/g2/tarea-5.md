# Tarea 5 · Comandos y contrato (G2-T2)

**Estado:** DONE
**Commits:** `b0ad666` (`feat(g2): comandos de archivos de prueba y su contrato`).

## Qué se implementó
- Comandos Tauri en `commands::fixtures`:
  - `fixture_add`: extrae `NewFixtureMeta` y bytes mediante `frame::split` del cuerpo binario raw, valida disponibilidad con `state.ensure_ready()` y delega a `repo::fixtures::add`.
  - `fixture_list`: lista los archivos de prueba de un caso con `repo::fixtures::list`.
  - `fixture_remove`: elimina el archivo de prueba física y lógicamente reordenando posiciones con `repo::fixtures::remove`.
  - `fixture_reveal`: obtiene la ruta absoluta con `repo::fixtures::absolute` y la revela en el explorador con `app.opener().reveal_item_in_dir`.
  - `fixture_copy_path`: obtiene la ruta absoluta con `repo::fixtures::absolute` y copia el texto al portapapeles del sistema con `app.clipboard().write_text`.
- Registro y ACL de comandos:
  - Exportados en `commands::mod::fixtures`.
  - Registrados en `commands::names::APP_COMMANDS` (`fixture_add`, `fixture_list`, `fixture_remove`, `fixture_reveal`, `fixture_copy_path`).
  - Registrados en `generate_handler!` en `src-tauri/src/lib.rs`.
  - Añadidas capacidades en `src-tauri/capabilities/main.json` (`allow-fixture-add`, `allow-fixture-list`, `allow-fixture-remove`, `allow-fixture-reveal`, `allow-fixture-copy-path`).
- DTOs y Contrato Rust ↔ TS:
  - `NewFixtureMeta`: estructura con `case_id`, `file_name`, `note` (`camelCase`, `deny_unknown_fields`).
  - `PackageExportResultDto`: agregado campo `missing_files: i64`, propagado desde `export.missing_files` en `commands::data::export_package`.
  - `RestoreResultDto`: agregado campo `missing_files: i64`, calculado con `repo::fixtures::missing_files` en `backup::restore`.
  - `FixtureDto`: expuesto como DTO de retorno para `fixture_add` y `fixture_list`.
  - `AppState::with_conn`: helper para ejecutar operaciones con la conexión SQLite.
  - Muestras de contrato en `dto::contract::data_samples()` actualizadas con `fixture`, `packageExportResult.missing_files: 0` y `restoreResult.missing_files: 0`.
  - Regenerado `src/infrastructure/tauri/__fixtures__/contracts.json` con `$env:UPDATE_CONTRACTS="1"; cargo test contract`.
- Contrato TypeScript y Dominio:
  - Esquema Zod `fixtureSchema` en `src/infrastructure/tauri/dto.ts`.
  - Agregado `missingFiles: z.number().int()` a `packageExportResultSchema` y `restoreResultSchema`.
  - Verificación en `src/infrastructure/tauri/contract.test.ts` con `["fixture", fixtureSchema]`.
  - `PackageExportResult` y `RestoreResult` en `src/domain/data.ts` con campo `missingFiles: number`.
  - Función `missingFilesMessage(n: number)` con manejo singular/plural.
  - Actualización de `restoreMessage` para incluir archivos de prueba/guion faltantes cuando `missingFiles > 0`.
  - En `data-slice.ts`, `exportPackage` emite toast con `missingFilesMessage` cuando `missingFiles > 0`.
  - Implementación en memoria (`MemoryData`) actualizada con `missingFiles: 0`.

## Archivos
- `src-tauri/src/commands/fixtures.rs` (creado)
- `src-tauri/src/commands/mod.rs` (modificado)
- `src-tauri/src/commands/names.rs` (modificado)
- `src-tauri/src/commands/data.rs` (modificado)
- `src-tauri/src/dto.rs` (modificado)
- `src-tauri/src/backup.rs` (modificado)
- `src-tauri/src/state.rs` (modificado)
- `src-tauri/src/lib.rs` (modificado)
- `src-tauri/capabilities/main.json` (modificado)
- `src/domain/data.ts` (modificado)
- `src/domain/data.test.ts` (modificado)
- `src/infrastructure/tauri/dto.ts` (modificado)
- `src/infrastructure/tauri/contract.test.ts` (modificado)
- `src/infrastructure/tauri/__fixtures__/contracts.json` (modificado)
- `src/infrastructure/memory/data.ts` (modificado)
- `src/ui/store/data-slice.ts` (modificado)
- `src/ui/store/data-slice.test.ts` (modificado)

## Pruebas
- RED:
  - `cargo test backup` falló por falta de campo `missing_files` en `RestoreResultDto`.
  - `npx vitest run src/infrastructure/tauri/contract.test.ts src/domain/data.test.ts src/ui/store/data-slice.test.ts` falló en 4 pruebas:
    - `fixture cumple el esquema estricto`: esquema no definido / fixture no presente.
    - `mensaje al restaurar`: falta texto de `missingFiles`.
    - `mensaje de archivos faltantes al exportar paquete`: función `missingFilesMessage` no existía.
    - `avisa si faltan archivos de prueba o del guion al exportar`: toast no emitido.
- GREEN:
  - `cargo test contract`: `inputs_deserialize_from_ts_shape`, `inputs_reject_unknown_fields` y `dto_contract_matches_fixture` pasaron y actualizaron el fixture.
  - `cargo test names`: 14 passed (verificando unicidad, handlers en `lib.rs` y capabilities en `main.json`).
  - `cargo test backup`: 19 passed (incluyendo `restore_keeps_fixtures_and_the_guion_copy` con `missing_files == 0`).
  - `cargo test`: 568 passed (564 lib, 4 bridge); 0 failed.
  - `npx vitest run`: 58 archivos, 408 pruebas pasadas sin errores.
- Lint y formato:
  - `cargo fmt --check`: limpio (código 0).
  - `cargo clippy --all-targets -- -D warnings`: 0 advertencias (código 0).
  - `npm run typecheck`: 0 errores de tipo (código 0).
  - `npm run lint`: 0 advertencias de ESLint (código 0).

## Autorrevisión
- Todos los comandos de fixtures validan `state.ensure_ready()?` para proteger el modo recuperación.
- El body binario de `fixture_add` se parsea mediante `frame::split` garantizando tipado estricto sin deserializar campos desconocidos.
- Las capacidades de Tauri y el contrato JSON están completamente sincronizados entre Rust y TypeScript.
- `missingFilesMessage` y `restoreMessage` respetan exactamente las reglas gramaticales singular/plural especificadas.

## Dudas o riesgos
- Ninguna. Se respetaron todas las interfaces para la integración con la Tarea 6.

## Revisión (la llena el controlador)
- Spec:
- Hallazgos y cómo se resolvieron
