# Handoffs · G Bloque 0 (fixture de guion y pico de trasplante)

**Plan:** [2026-11-10-g0-fixture-y-pico](../../plans/2026-11-10-g0-fixture-y-pico.md) · **Spec:** [guion-a-evidencias](../../specs/2026-11-10-guion-a-evidencias-design.md) · **Contrato:** [G](../../contracts/G-guion-a-evidencias.md) · **Rama:** `feat/g0-fixture-y-pico`

Esta carpeta es la memoria versionada del bloque. Cualquier agente que tome el siguiente plan (G1, G2, G4) copia este formato a `handoffs/<bloque>/`.

## Cómo se trabaja

1. Un **controlador** lee el plan y despacha una task a la vez (superpowers:subagent-driven-development).
2. Un **implementador** (subagente nuevo por task) hace la task con TDD, la commitea y escribe su reporte en `tarea-<N>.md` de esta carpeta.
3. Un **revisor** (otro subagente) revisa el diff de la task contra el plan y el spec. El controlador anota el veredicto al final de `tarea-<N>.md`.
4. Si hay hallazgos Critical o Important, el implementador los corrige (hasta 5 rondas) y agrega su informe de corrección al mismo archivo.
5. Al terminar todas las tasks, una revisión de toda la rama con el modelo más capaz.

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
- Lint: `cargo fmt --check` y `cargo clippy --all-targets -- -D warnings`
## Autorrevisión
## Dudas o riesgos

## Revisión (la llena el controlador)
- Spec: ✅ / ❌ · Calidad: Aprobada / Necesita correcciones
- Hallazgos y cómo se resolvieron
## Correcciones (ronda <R>)
- Qué cambió, pruebas que lo cubren, comando y salida
```

Reglas del reporte: sin rutas personales, contraseñas ni datos de clientes; los comandos se escriben como se ejecutaron (desde `src-tauri/`).

## Qué revisa el revisor

**Cumplimiento del plan**
- Están todos los pasos de la task: archivos, firmas exactas, constantes y mensajes fijos tal como los da el plan.
- No hay nada de más (YAGNI): ni funciones públicas extra ni dependencias nuevas.
- Las pruebas del plan están con sus aserciones; se pueden agregar, no quitar ni debilitar.

**Restricciones globales del bloque**
- Sin dependencias nuevas en `Cargo.toml`; `calamine` sigue solo en `[dev-dependencies]`.
- Ningún archivo real de un cliente en el repo; los fixtures se generan en memoria.
- Mensajes al usuario en español, idénticos a los del plan.
- `cargo fmt --check`, `clippy -D warnings` y las pruebas de la task en verde, con salida limpia.

**Riesgos propios de este bloque**
- Todo lo que no es una de las tres partes índice (`xl/workbook.xml`, `xl/_rels/workbook.xml.rels`, `[Content_Types].xml`) queda byte a byte igual.
- La hoja injertada no lleva `t="s"`, ` s="`, `style=`, `customFormat` ni `tabSelected`.
- No se pisan `drawingN`, `imageN` ni `sheetN` existentes, y no se duplican los `Default` de `[Content_Types].xml`.
- Los nombres de hoja se escapan en el XML y se validan (vacío, más de 31 caracteres, `[ ] : * ? / \`).
- Los errores de `zip` y de UTF-8 no se tragan: terminan en `AppError` con mensaje claro.

**Severidad**
- Critical: rompe el libro o pierde datos.
- Important: requisito faltante, comportamiento frágil, prueba que no comprueba nada, errores tragados.
- Minor: estilo y cobertura que se podría ampliar (se anota y no frena la task).

## Registro

| Tarea | Estado | Commits | Revisión |
|---|---|---|---|
| 1 · Guion sintético | ✅ | `d032ece` | Spec ✅ · Calidad aprobada · 2 Minor |
| 2 · Partes del xlsx | ✅ | `f1febf0`, `d5f808f` | Spec ✅ · 1 Important corregido en ronda 1 · 4 Minor |
| 3 · Trasplante de hoja | ✅ | `93a62c3` | Spec ✅ · Calidad aprobada · 6 Minor (endurecimiento para G4) |
| 4 · Verificación manual y go / no-go | ✅ | `6f0c962` + este commit | **go** · Excel y LibreOffice sin aviso ([pico-trasplante](pico-trasplante.md)) |
