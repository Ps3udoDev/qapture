# ADR 0004: Tests de contrato en lugar de `tauri-specta`

**Fecha:** 2026-10-15 · **Estado:** vigente · Decisión D12 del [spec](../superpowers/specs/2026-10-15-qapture-design.md).

## Contexto

Los tipos viajan entre Rust y TypeScript por `invoke()`. Si un lado cambia sin el otro, el error aparece en ejecución. `tauri-specta` generaría los tipos, pero seguía en versión candidata (RC) al tomar la decisión.

## Decisión

Rust serializa DTO de ejemplo a `src/infrastructure/tauri/__fixtures__/contracts.json` y el front los valida con esquemas `zod` estrictos. Si un lado cambia sin el otro, falla un test. El fixture solo se regenera con `UPDATE_CONTRACTS=1 cargo test contract`, y CI verifica que no cambie por sorpresa.

## Consecuencias

- Los desajustes se detectan en CI, no en producción.
- Cada DTO se declara dos veces (struct en Rust, esquema `zod` en TS); es más manual que generar tipos.
- En ejecución, el adaptador además valida cada respuesta con `zod` y falla con un error de contrato claro.
- Se puede reevaluar `tauri-specta` cuando salga de RC.
