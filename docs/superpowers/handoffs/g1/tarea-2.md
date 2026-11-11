# Tarea 2 · Perfil de importación (G1-T2)

**Estado:** DONE
**Commits:** ver `git log` (`feat(g1): perfil de importación del guion`).

## Qué se implementó
- Tipos JSON v1 del perfil con serde, nombres camelCase y rechazo de propiedades desconocidas, incluidos los objetos anidados.
- Equivalencias de estado por perfil y casilla Condicionado, comparación de opciones de lista y validación estructural con los mensajes del plan.
- `domain::text::fold` comparte el mapa de acentos con `slugify`, normaliza mayúsculas y colapsa espacios internos.
- Perfil de referencia reutilizable en pruebas: las 13 columnas del fixture, diez campos, roles y carátula D5/D6/D8.

## Archivos
- `src-tauri/src/transfer/guion/profile.rs`.
- `src-tauri/src/transfer/guion/mod.rs`.
- `src-tauri/src/domain/text.rs`.

## Pruebas
Todos los comandos desde `src-tauri/`.
- RED: `cargo test transfer::guion::profile` → 42 errores de compilación por `ProfileDefinition`, `Target`, `FieldRole`, `map_status`, `problems` y demás tipos/funciones inexistentes. Las cuatro pruebas del plan se escribieron antes de la implementación.
- GREEN inicial: `cargo test transfer::guion::profile` → 4 passed, 0 failed.
- GREEN final: `cargo test transfer::guion::profile` → 6 passed, 0 failed. Se añadieron guardas para tipos no admitidos, claves/roles repetidos, clave reservada y propiedades desconocidas anidadas.
- Regresión y fold: `cargo test domain::text` → 3 passed, 0 failed; las pruebas previas de `slugify` permanecen intactas.
- Lint: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `git diff --check` → exit 0, sin errores.
- El mensaje previo `linker_messages` de MSVC al enlazar la DLL aparece en `cargo test`; clippy termina limpio.

## Autorrevisión
- Se conservan los seis estados de Qapture y el marcador `Aprobado+condicionado` se transforma en Aprobado con booleano, sin agregar un estado.
- El perfil solo admite text, longtext, list y date. No se permiten boolean ni number, conforme a P5.
- `reference()` usa los encabezados y el nombre de hoja del fixture de pruebas; no hay datos específicos de cliente en producción.
- El helper `FieldMut` necesita desreferenciar `role` en la prueba del plan (`*field_mut(...).role = ...`) porque es una referencia mutable; las aserciones se conservan.
- Los filtros de pruebas se ejecutaron por separado porque cargo admite un único filtro posicional.

## Dudas o riesgos
- Se conserva el `allow(dead_code)` transitorio del módulo, introducido en T1 y pendiente de retirar al conectar T6.
- La comparación aplica el mapa de acentos existente en `slugify`; no se incorpora una dependencia de normalización Unicode.

## Revisión (la llena el controlador)
