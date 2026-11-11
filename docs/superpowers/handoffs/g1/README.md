# Handoffs · G Bloque 1 (importar guion de pruebas)

**Plan:** [2026-11-10-g1-importar-guion](../../plans/2026-11-10-g1-importar-guion.md) · **Spec:** [guion-a-evidencias](../../specs/2026-11-10-guion-a-evidencias-design.md) · **Contrato:** [G](../../contracts/G-guion-a-evidencias.md) · **Rama:** `feat/g1-importar-guion`

Esta carpeta es la memoria versionada del bloque. Mantiene el formato de [G0](../g0/README.md).

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

## Qué revisa el revisor

**Cumplimiento del plan**
- Archivos, firmas, constantes y mensajes exactos.
- Sin funciones ni dependencias ajenas al alcance.
- Aserciones del plan completas, sin debilitarlas.

**Restricciones globales**
- La única dependencia modificada es `calamine = "0.36.1"`: pasa a producción sin features. Se conserva el parche `zip8`.
- Ningún archivo real de cliente; el fixture se genera en memoria.
- Límites: 50 MB, 10000 casos, inspección de 200 filas × 52 columnas.
- Pruebas, formato, clippy, typecheck y lint según cada tarea.

**Riesgos propios del bloque**
- Proyecto, perfil opcional, origen y copia se importan todo o nada.
- `project_source.definition` sobrevive al borrado del perfil.
- Estados y listas se comparan sin acentos, mayúsculas ni espacios adicionales.
- Fechas normalizadas y avisos para fechas inválidas.
- El webview usa un token, nunca rutas del archivo elegido.
- Códigos vacíos o repetidos bloquean la importación.
- G2 y G4 siguen fuera de alcance, incluida la reimportación de paquetes con códigos arbitrarios.

**Severidad**
- Critical: pérdida de datos o libro roto.
- Important: requisito faltante, comportamiento frágil, prueba ineficaz o errores tragados.
- Minor: estilo o cobertura ampliable; se documenta sin frenar la tarea.

## Registro

| Tarea | Estado | Commits | Revisión |
|---|---|---|---|
| 1 · Lector de guion | ✅ | `806a186` | Spec ✅ · Calidad aprobada · 1 Minor |
| 2 · Perfil de importación | ✅ | `ed09680` | Spec ✅ · Calidad aprobada · 1 Minor |
| 3 · Migración y repositorios | ✅ | `248d731` | Spec ✅ · 1 Important corregido en ronda 1 |
| 4 · Vista previa | ✅ | `fe9ea84` | Spec ✅ · 1 Important corregido en ronda 1 |
| 5 · Importación transaccional | ✅ | `361397d` | Spec ✅ · Calidad aprobada |
| 6 · Comandos y contrato | Pendiente | — | Pendiente |
| 7 · Puerto y estado | Pendiente | — | Pendiente |
| 8 · Asistente y cierre | Pendiente | — | Pendiente |

## Notas del controlador

- La rama solicitada ya existía, basada en `main` con G0 integrado y el plan G1. El árbol inicial estaba limpio.
- La skill `superpowers:subagent-driven-development` no está instalada. Se aplica el flujo de implementador y revisor por tarea documentado arriba.
- Los commits usan la autoría real de Git; no se agrega una atribución a Claude porque esta ejecución utiliza agentes Codex.
- Las comprobaciones manuales permanecen pendientes hasta contar con evidencia de su ejecución.
