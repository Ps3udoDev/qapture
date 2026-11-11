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
| 1 · Restaurar respaldos anteriores a las tablas del guion | DONE | `b399c5f` | Pendiente |
| 2 · Almacén en disco de archivos de prueba (G2-T1 p1) | DONE | `39bd023` | Pendiente |
| 3 · Repositorio de archivos de prueba (G2-T1 p2) | DONE | `89287b7` | Pendiente |
| 4 · Paquete .qapture formato 2 (G2-T3) | DONE | `3e5403b` | Pendiente |
| 5 · Comandos y contrato (G2-T2) | DONE | `b0ad666` | Pendiente |
| 6 · Puerto, adaptadores y estado (G2-T4 p1) | DONE | `28170d9` | Pendiente |
| 7 · Sección «Archivos de prueba» y cierre (G2-T4 p2) | Pendiente | - | - |
