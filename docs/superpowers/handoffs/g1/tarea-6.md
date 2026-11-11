# Tarea 6 · Comandos y contrato (G1-T6)

**Estado:** DONE_WITH_CONCERNS
**Commits:** ver `git log` (`feat(g1): comandos del asistente de guion y su contrato`).

## Qué se implementó
- Cinco comandos Tauri del asistente: inspección con selector nativo, vista previa, importación, listado y borrado de perfiles. Registrados en `names.rs`, `generate_handler!` y capability de la ventana principal.
- El archivo permanece en `AppState` con token aleatorio. La interfaz recibe nombre y muestras, nunca la ruta elegida. Token inexistente o distinto devuelve el mensaje fijo. Importar correctamente libera el libro pendiente; un error conserva el contexto para corregir el formulario.
- El selector limita a `.xlsx`; la lectura se limita a 50 MB tanto por metadatos como por bytes efectivamente leídos, cubriendo que el archivo crezca entre ambas operaciones. Todas las operaciones exigen que la base esté fuera del modo recuperación.
- Inspección limitada a 200 filas × 52 columnas por hoja visible, fila sugerida, carátula y primer perfil cuyo nombre de hoja y encabezados coinciden según `fold`.
- DTOs de inspección e input estricto, esquemas Zod estrictos y fixture regenerado desde los DTOs Rust usando el guion sintético de G0. Targets discriminados por `kind`, roles anulables y los cuatro tipos del perfil acordados en P5.
- Se retiraron los `allow(dead_code)` de los módulos `transfer::guion` y `repo::guion`; quedan anotaciones locales para los lectores de origen que consumirá G4 y el mapa de estados de referencia que usa el contrato y refleja TypeScript.

## Archivos
- `src-tauri/src/commands/guion.rs`, `commands/data.rs`, `commands/mod.rs`, `commands/names.rs`.
- `src-tauri/src/state.rs`, `dto.rs`, `lib.rs`, `capabilities/main.json`.
- `src-tauri/src/repo/guion.rs`, `repo/mod.rs`, `transfer/mod.rs`, `transfer/guion/profile.rs` (solo anotaciones locales de código aún sin consumidor de producción).
- `src/infrastructure/tauri/dto.ts`, `contract.test.ts`, `__fixtures__/contracts.json`.

## Pruebas
Desde `src-tauri/`:
- RED: `cargo test commands::guion` → 11 errores de compilación por `PendingGuion`, `GuionImportInput`, `suggested_profile` y métodos del estado inexistentes. Las pruebas de estado y DTO se compilan también con este filtro.
- Regeneración: `$env:UPDATE_CONTRACTS='1'; cargo test contract; Remove-Item Env:UPDATE_CONTRACTS` → 3 passed. Fixture generado por Rust; no se editó a mano.
- GREEN: `cargo test` → biblioteca 538 passed, 0 failed, 4 ignored; integración `tests/bridge.rs` 4 passed, 0 failed; binarios y doc-tests sin fallos. Incluye pruebas de comandos/permisos, token, DTO y toda la regresión anterior.
- Añadida `inspection_limits_rows_and_columns_and_handles_missing_headers`: inspección de 210 × 60 celdas numéricas produce 200 × 52, sin encabezados ni carátula inventados.
- Formato: `cargo fmt --check` → exit 0.
- Clippy: `cargo clippy --all-targets -- -D warnings` → exit 0, sin advertencias.

Desde la raíz:
- RED: `npx vitest run src/infrastructure/tauri/contract.test.ts` → 4 failed, 38 passed: los tres esquemas nuevos y el esquema de perfil aún no existían.
- GREEN: mismo comando → 43 passed, 0 failed. Además de muestras y claves del input, comprueba rechazo de propiedades ajenas en perfil, target y DTOs de guion.
- `npm run typecheck`, `npm run lint` y `git diff --check` → exit 0.

## Autorrevisión
- El mutex del guion se libera antes de `clear_guion`, evitando un bloqueo recursivo al terminar el import. La base solo se bloquea durante operaciones síncronas y nunca durante el diálogo de archivo.
- No hay rutas entrantes en los cinco comandos. La copia y la inserción usan el import transaccional aprobado en T5.
- La sugerencia de perfil usa todas sus columnas mapeadas y rechaza definiciones sin columnas o con fila cero para evitar sugerencias vacías o desbordamientos.
- T7 aún no existía al implementar esta tarea: el contrato de input TS usa `GuionImportInputDto = z.infer<...>`; el dominio de T7 tendrá la misma estructura.
- Ninguna dependencia nueva. Ningún archivo real de cliente. Las cuatro pruebas ignoradas son comprobaciones previas del proyecto; no se ignoró ninguna prueba nueva.

## Dudas o riesgos
- Minor conocido por el revisor/controlador: el import valida/usa el token bajo un candado y después limpia el pendiente en otro. El flujo de interfaz de T7 debe usar `dataBusy` exclusivo como pide el plan; llamadas IPC concurrentes ajenas a ese flujo podrían sustituir el pendiente entre ambas operaciones. No se agregó una nueva API fuera de las interfaces previstas.
- `default_status_map` es una referencia pública del perfil exigida en T2 y usada por pruebas/contrato; en producción el mapa llega del asistente TS. Conserva un `allow(dead_code)` local documentado.
- El enlazador MSVC emite su mensaje previo `linker_messages` al compilar la DLL; las comprobaciones de código no producen advertencias.

## Revisión (la llena el controlador)
