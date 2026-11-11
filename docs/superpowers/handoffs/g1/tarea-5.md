# Tarea 5 · Importar todo o nada (G1-T5)

**Estado:** DONE_WITH_CONCERNS
**Commits:** ver `git log` (`feat(g1): importar el guion como proyecto, todo o nada`).

## Qué se implementó
- `BundleSource`, `ProjectBundle.source` y `BundleCase.source_row`; el insert del proyecto/caso conserva el origen y su fila de Excel.
- `bundle::insert_in` contiene exclusivamente la escritura de filas. `bundle::insert` conserva su transacción, la confirmación de archivos y la aplicación de GitHub después del commit, con los mismos avisos.
- `ImportFiles::write_file` escribe una copia con `write_atomic`, registra la carpeta para rollback y devuelve su ruta relativa. `commit` queda público para el importador de guion.
- `import::import` valida la vista previa y el nombre, guarda la copia `guion.xlsx`, calcula SHA-256 hexadecimal y confirma perfil opcional, proyecto, campos, casos y origen en una transacción. Si falla, revierte filas y elimina la copia.
- Importa sin plantilla, con metadatos vacíos y secuencia nueva `EJ-001`; mantiene los códigos originales y los anchos por tipo del plan. Propaga los avisos de la vista previa.
- Legacy y paquetes inicializan el origen opcional como `None`, sin alterar su comportamiento.

## Archivos
- `src-tauri/src/transfer/bundle.rs`.
- `src-tauri/src/transfer/guion/import.rs`.
- `src-tauri/src/transfer/legacy.rs`.
- `src-tauri/src/transfer/package.rs`.

## Pruebas
Comandos desde `src-tauri/`.
- RED: `cargo test transfer::guion::import` → 5 errores de compilación: faltan `ImportRequest` y `import`. Se escribieron primero las tres pruebas exigidas en el plan, conservando sus aserciones.
- GREEN inicial: `cargo test transfer::` → 64 passed, 0 failed; comprende los tres casos nuevos y la regresión de legacy, paquete y datos GitHub.
- GREEN ampliado: `cargo test transfer::` → 66 passed, 0 failed. Dos pruebas adicionales verifican fallo SQL después de insertar el perfil/proyecto (rollback de filas, campos y copia), y el import sin guardar perfil (definición intacta, avisos, sufijo de nombre y anchos).
- Lint: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `git diff --check` → exit 0, sin diferencias ni errores.
- MSVC emite el mensaje previo `linker_messages` al enlazar la DLL durante `cargo test`; no es un aviso del código importador.

## Autorrevisión
- Las aserciones existentes de `bundle.rs` no se modificaron. Las pruebas de GitHub siguen comprobando que sus fallos se convierten en avisos después de confirmar el proyecto.
- Ninguna dependencia nueva ni archivo real de cliente. Las pruebas generan el guion sintético en memoria.
- El límite de errores usa `bundle::Problems`, incluido el encabezado `IMPORT_FAILED` y las viñetas.
- El nombre de archivo de `write_file` rechaza separadores, `:` y nombres `.`/`..`; el importador usa únicamente `GUION_FILE` y un id de proyecto recién generado.

## Dudas o riesgos
- Se conserva el `allow(dead_code)` transitorio en los módulos de guion: aún faltan los consumidores T6 de listado/borrado y comandos; `source` y `case_rows` tendrán uso de producción en G4. No se añade ningún `allow` nuevo.
- Los paquetes con códigos de guion siguen fuera de alcance hasta G2, como indica el plan.

## Revisión (la llena el controlador)
