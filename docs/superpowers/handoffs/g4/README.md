# Handoffs · G Bloque 3 (formato de origen)

**Plan:** [2026-11-11-g4-formato-de-origen](../../plans/2026-11-11-g4-formato-de-origen.md) · **Spec:** [guion-a-evidencias](../../specs/2026-11-10-guion-a-evidencias-design.md) · **Contrato:** [G](../../contracts/G-guion-a-evidencias.md) · **Rama:** `feat/g4-formato-de-origen`

Esta carpeta es la memoria versionada del bloque. Mantiene el formato de [G0](../g0/README.md), [G1](../g1/README.md) y [G2](../g2/README.md).

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
| 1 · Escritura de celdas (G4-T1) | DONE | `c1875b0` | Aprobada |
| 2 · Valores por fila y carátula (G4-T2) | DONE | `6b6c4fc` | Aprobada (revisión de rama) |
| 3 · Hoja «Evidencias» injertada (G4-T3) | DONE | `53b3ccb` | Aprobada (revisión de rama) |
| 4 · Exportación completa y prueba de ida y vuelta (G4-T4) | DONE | `6d18041` | Aprobada (revisión de rama) |
| 5 · Comandos y contrato (G4-T5) | DONE | `78277f4` | Aprobada (revisión de rama) |
| 6 · Opción «Formato de origen» en el diálogo de exportar (G4-T6) | DONE | `655eb84` | Aprobada (revisión de rama) |
| 7 · Cierre del bloque | DONE | `3e7f817` | Aprobada (revisión de rama) |

## Revisión de la rama

**Veredicto:** aprobada (sin hallazgos Critical ni Important). Diff completo `main..feat/g4-formato-de-origen` revisado contra el plan, el spec y los tests del «Review Focus»; todos existen y pasan.

### Verificación manual (`guion_ejecutado.xlsx`)

Generado mediante el test ignorado `write_origin_for_manual_check` en `target/manual-check/guion_ejecutado.xlsx` (a partir de `guion_fixture::build()`, 4 casos ejecutados y carátula diligenciada).

| Comprobación | Esperado | Excel | LibreOffice |
|---|---|---|---|
| Aviso de reparación al abrir | Ninguno | ✅ Ninguno | ✅ Ninguno |
| Pestañas | «Guión de Pruebas», «Evidencias» (Hoja1 sigue oculta) | ✅ | ✅ |
| Hoja del guion (I, K, L, M) | Celdas de destino modificadas según perfil:<br/>- CP01: Aprobado (K16 = «Aceptado»), fecha de evidencia en I16, tester en L16.<br/>- CP02: Fallido (K17 = «Rechazado»), tester en L17.<br/>- CP03: Aprobado condicionado (K18 = «Condicionado»), tester en L18.<br/>- CP04: Bloqueado (M19 = «Bloqueado: Sin ambiente», K19 = «»), tester en L19.<br/>Filas 20 a 51 intactas. | ✅ | ✅ |
| Carátula (D5, D6, D8) | D5 = fecha máxima (10/11/2026), D6 = «Ana Tester», D8 = «Ejecución de prueba del bloque G4.» | ✅ | ✅ |
| Hoja «Evidencias» | Secciones para CP01, CP02, CP03, CP04. CP01 con 2 capturas insertadas y vínculo «Volver al guion» apuntando a `'Guión de Pruebas'!A16` | ✅ | ✅ |
| Guardar y volver a abrir | Sin aviso de reparación; todas las hojas, estilos, fórmulas y datos se conservan | ✅ | ✅ |

Verificación completa del bloque:
- `cargo fmt --check` ✅
- `cargo clippy --all-targets -- -D warnings` ✅
- `cargo test` ✅
- `npm run lint` ✅
- `npm run typecheck` ✅
- `npx vitest run` ✅
