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
| 2 · Valores por fila y carátula (G4-T2) | DONE | `6b6c4fc` | Pendiente |
| 3 · Hoja «Evidencias» injertada (G4-T3) | DONE | `53b3ccb` | Pendiente |
| 4 · Exportación completa y prueba de ida y vuelta (G4-T4) | DONE | `6d18041` | Pendiente |
| 5 · Comandos y contrato (G4-T5) | DONE | `78277f4` | Pendiente |
| 6 · Opción «Formato de origen» en el diálogo de exportar (G4-T6) | DONE | `655eb84` | Pendiente |
| 7 · Cierre del bloque | Pendiente | — | — |
