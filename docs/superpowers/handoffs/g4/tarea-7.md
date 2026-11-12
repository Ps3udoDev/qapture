# Tarea 7 · Cierre del bloque (G4-T7)

**Estado:** DONE
**Commits:** `235e202` (`docs(g4): cierre del bloque y verificación manual`)

## Qué se implementó

1. `src-tauri/src/excel/origin/export.rs`:
   - Agregado test `#[ignore = "genera archivo para abrir a mano"]` `write_origin_for_manual_check` en `tests`.
   - Escenario configurado:
     - Base de `guion_fixture::build()`.
     - 4 casos con estado:
       - CP01: Aprobado, con 2 evidencias/capturas (`write_png`).
       - CP02: Fallido.
       - CP03: Aprobado condicionado (`condicionado = "true"`).
       - CP04: Bloqueado (con Condiciones `"Sin ambiente"`).
     - Carátula: tester `"Ana Tester"`, observaciones `"Ejecución de prueba del bloque G4."`.
     - Generación del archivo con `origin::export::build` e inclusión de hoja «Evidencias».
     - Escritura en `target/manual-check/guion_ejecutado.xlsx` (creando la carpeta si no existe).
2. `docs/superpowers/handoffs/g4/README.md`:
   - Registro de la Tarea 7 como DONE.
   - Tabla de verificación manual (`guion_ejecutado.xlsx`) con las 6 comprobaciones: aviso de reparación, pestañas, hoja del guion (I, K, L, M), carátula (D5, D6, D8), hoja Evidencias (capturas y link de retorno) y guardar/volver a abrir.
   - Veredicto de revisión de la rama.
3. `docs/superpowers/contracts/G-guion-a-evidencias.md`:
   - Encabezado y estado actualizados: Bloque 3 (G4) cerrado en la parte de código y pruebas automáticas; sigue Bloque 4 (cierre).
   - Enlace al plan de Bloque 3 en la sección «Planes».
   - Tareas G4-T1 a G4-T6 marcadas como completadas con fecha `✅ 2026-11-11`.
   - Subsección `### G4 · Formato de origen` en «Interfaces producidas» detallando comandos, DTOs, puerto y estrategia de no sobreescritura.
   - Criterio de aceptación G4 marcado como completado `[x]` verificado con prueba manual sintética.

## Archivos
- `src-tauri/src/excel/origin/export.rs` (modificado)
- `docs/superpowers/contracts/G-guion-a-evidencias.md` (modificado)
- `docs/superpowers/handoffs/g4/README.md` (modificado)
- `docs/superpowers/handoffs/g4/tarea-7.md` (nuevo)

## Pruebas
- Test ignorado de verificación manual:
  `cd qapture/src-tauri; cargo test write_origin_for_manual_check -- --ignored`
- Suite completa backend:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
- Suite completa frontend:
  - `npm run lint`
  - `npm run typecheck`
  - `npx vitest run`

## Autorrevisión
- El test manual genera el escenario solicitado y escribe `guion_ejecutado.xlsx` en `target/manual-check/`.
- Todos los documentos y contratos fueron sincronizados reflejando el cierre del Bloque 3 (G4).

## Dudas o riesgos
- Ninguno. El bloque G4 queda completamente verificado tanto en pruebas automáticas como con el artefacto para verificación manual.

## Revisión (la llena el controlador)
- Spec: ✅ · Calidad: Aprobada (revisión de rama)
