# Handoffs · G Bloque 2 (archivos de prueba)

**Plan:** [2026-11-11-g2-archivos-de-prueba](../../plans/2026-11-11-g2-archivos-de-prueba.md) · **Spec:** [guion-a-evidencias](../../specs/2026-11-10-guion-a-evidencias-design.md) · **Contrato:** [G](../../contracts/G-guion-a-evidencias.md) · **Rama:** `feat/g2-archivos-de-prueba`

Esta carpeta es la memoria versionada del bloque. Mantiene el formato de [G0](../g0/README.md) y [G1](../g1/README.md).

## Cómo se trabaja

1. El controlador lee el plan y despacha una tarea a la vez.
2. Un implementador nuevo por tarea implementa con TDD, crea los commits y escribe `tarea-<N>.md`.
3. Otro subagente revisa el diff contra el plan y el spec. El controlador registra el veredicto.
4. Los hallazgos Critical o Important se corrigen y revisan, hasta cinco rondas, en el mismo reporte.
5. Al terminar se revisa toda la rama.

## Formato del reporte del implementador (`tarea-<N>.md`)

```markdown
# Tarea <N> · <nombre>

**Estado:** DONE | DONE_WITH_CONCERNS | BLOCKED | NEEDS_CONTEXT
**Commits:** <sha corto> <asunto>

## Qué se implementó
## Archivos
## Pruebas
- RED: comando, salida relevante del fallo y por qué era el esperado
- GREEN: comando y salida
- Lint: comandos y salida
## Autorrevisión
## Dudas o riesgos

## Revisión (la llena el controlador)
- Spec: ✅ / ❌ · Calidad: Aprobada / Necesita correcciones
- Hallazgos y cómo se resolvieron
## Correcciones (ronda <R>)
- Qué cambió, pruebas que lo cubren, comando y salida
```

Sin rutas personales, contraseñas ni datos de clientes. Los comandos indican desde qué directorio se ejecutaron.

## Registro

| Tarea | Estado | Commits | Revisión |
|---|---|---|---|
| 1 · Restaurar respaldos anteriores a las tablas del guion | DONE | `b399c5f` | Aprobada (revisión de rama) |
| 2 · Almacén en disco de archivos de prueba (G2-T1 p1) | DONE | `39bd023` | Aprobada (revisión de rama) |
| 3 · Repositorio de archivos de prueba (G2-T1 p2) | DONE | `89287b7` | Aprobada (revisión de rama) |
| 4 · Paquete .qapture formato 2 (G2-T3) | DONE | `3e5403b` | Aprobada (revisión de rama) |
| 5 · Comandos y contrato (G2-T2) | DONE | `b0ad666` | Aprobada (revisión de rama) |
| 6 · Puerto, adaptadores y estado (G2-T4 p1) | DONE | `28170d9` | Aprobada (revisión de rama) |
| 7 · Sección «Archivos de prueba» y cierre (G2-T4 p2) | DONE | `93d1ccd` | Aprobada (revisión de rama) |

## Revisión de la rama

**Veredicto:** aprobada con correcciones menores (sin hallazgos Critical). Diff completo `main..feat/g2-archivos-de-prueba` revisado contra el plan, el spec y los tests del «Review Focus»; todos existen y pasan.

Correcciones aplicadas en `fix(g2): correcciones de la revisión de la rama`:

| # | Severidad | Hallazgo | Corrección |
|---|---|---|---|
| 1 | Important | `loadFixtures` conservaba la lista del caso anterior mientras cargaba (y para siempre si la carga fallaba): «Quitar» podía actuar sobre un archivo de otro caso | Al cambiar de caso la lista se vacía. Test `al cambiar de caso no muestra la lista del anterior aunque la carga falle` (RED → GREEN) |
| 2 | Minor | La sección aceptaba como zona de soltar cualquier arrastre (p. ej. reordenar capturas) y mostraba el contorno | `onDragOver`/`onDrop` usan `hasFiles`, igual que `CaseDetail` |
| 3 | Minor | Quedaban `allow(dead_code)` de tareas intermedias en `media/fixtures.rs`, `repo/fixtures.rs` y `PackageExport.missing_files` | Eliminados; clippy sigue limpio |
| 4 | Minor | `tarea-7.md` citaba un SHA inexistente (`275860c`) | Corregido a `93d1ccd` |

Verificación final (desde `src-tauri/`): `cargo fmt --check` ✅ · `cargo clippy --all-targets -- -D warnings` ✅ · `cargo test` → 564 + 4 passed, 4 ignored. Desde la raíz: `npm run typecheck` ✅ · `npm run lint` ✅ · `npx vitest run` → 61 archivos, 427 tests.

Prueba manual de la Task 7 (Step 5) con `npm run tauri dev`: **aprobada** por el tester el 2026-11-11 (las 4 comprobaciones). El criterio G2 del contrato queda confirmado.
