# Tarea 6 · Opción «Formato de origen» en el diálogo de exportar (G4-T6)

**Estado:** DONE
**Commits:** `655eb84` (`feat(g4): opción Formato de origen en el diálogo de exportar`)

## Qué se implementó

1. `src/domain/export.ts`:
   - `ExportKind`: ampliado con `"origin"` (`"project" | "template" | "origin"`).
   - Interfaces de dominio:
     - `OriginInfo`: `{ fileName, sheet, casesWritten, unmapped, evidence, formulas, unavailable, suggestedName }`.
     - `OriginExportOptions`: `{ tester, observations, includeEvidence, imageMaxWidth }`.
     - `defaultOriginExportOptions(tester)`: inicializa con observaciones vacías, evidencias incluidas y ancho por defecto.
   - Mensajes fijos:
     - `unmappedMessage(codes)`: singular (`1 caso no tiene fila en el guion y no se escribe: <código>.`) y plural (`<n> casos no tienen fila en el guion y no se escriben: <códigos>.`), o `null` si vacío.
     - `formulasMessage(cells)`: `Las celdas <lista> tenían fórmula: se escribe el valor y se quita la fórmula.`, o `null` si vacío.
   - Validación:
     - `validateOriginExportOptions(options)`: valida `tester` obligatorio (máx 100 caracteres), `observations` (máx 500 caracteres, mensaje `"La observación supera 500 caracteres."`) y `imageMaxWidth` entre 400 y 1600 px.

2. `src/ports/export.ts`:
   - Métodos en la interfaz `Exporter`:
     - `originInfo(projectId: string): Promise<OriginInfo | null>;`
     - `exportOrigin(projectId: string, options: OriginExportOptions): Promise<ExportResult | null>;`

3. Adaptadores de infraestructura:
   - `src/infrastructure/tauri/export.ts`:
     - `originInfo`: llama al comando `"export_origin_info"` con `{ projectId }` y valida con `originInfoSchema.nullable()`.
     - `exportOrigin`: llama al comando `"export_origin"` con `{ projectId, input: options }` y valida con `exportResultSchema.nullable()`.
   - `src/infrastructure/memory/export.ts`:
     - `MemoryExporter` mantiene `originInfoValue: OriginInfo | null`, método `setOriginInfo(info)` para tests y soporte para `{ kind: "origin", projectId, options }` en `calls`.
     - `originInfo`: devuelve `originInfoValue`.
     - `exportOrigin`: valida simulación de bloqueo (`locked`), cancelación (`cancel`), emite progreso y devuelve `ExportResult` mock con el nombre sugerido.

4. `src/application/use-cases.ts`:
   - `exportOrigin(repos, projectId, options, onProgress)`: valida las opciones de origen con `validateOriginExportOptions`, suscribe el manejador de progreso, ejecuta `repos.exporter.exportOrigin`, y maneja excepciones mapeando `file_locked` y errores de validación a `errors._form`.

5. `src/ui/store/export-slice.ts` & `src/ui/store/workspace.ts`:
   - Estado en `ExportSlice`: `originInfo: OriginInfo | null`.
   - Acciones:
     - `loadOriginInfo()`: invoca `repos.exporter.originInfo(projectId)` y actualiza el estado.
     - `exportOrigin(options)`: orquesta la exportación, limpia el estado de avance al terminar y publica toast de éxito `Guion ejecutado guardado: <fileName>` con acciones «Abrir archivo» y «Abrir carpeta», o aviso si hubo imágenes faltantes.
   - En `workspace.ts`: la suscripción al cambio de proyecto invoca tanto `loadDocVersions()` como `loadOriginInfo()`.

6. `src/ui/screens/ExportDialog.tsx`:
   - Al montar, refresca `loadOriginInfo()`.
   - `KINDS`: incluye `"origin"` («Formato de origen», «El guion del cliente con resultados y evidencias.») únicamente si `originInfo !== null`.
   - Si `originInfo?.unavailable`: opción de radio deshabilitada mostrando el motivo del bloqueo.
   - Cuando se selecciona `origin`:
     - Oculta el selector de versiones de documento y la casilla de hoja Resumen.
     - Muestra detalles del archivo de origen: nombre del archivo, hoja, `<n> casos se escriben`, `<n> evidencias`.
     - Si hay casos sin fila: muestra `unmappedMessage(originInfo.unmapped)`.
     - Si hay fórmulas quitadas: muestra `formulasMessage(originInfo.formulas)`.
     - Muestra el aviso fijo de privacidad: `"Revisa que las capturas no muestren datos personales o del cliente antes de entregar el archivo."`.
     - Casilla `"Agregar hoja Evidencias"` (marcada por defecto).
     - Campo de texto textarea `"Observaciones de la carátula"` (límite 500 caracteres).
     - Botón de submit con texto `"Exportar guion"`.

## Archivos
- `src/domain/export.ts` (modificado)
- `src/domain/export.test.ts` (modificado)
- `src/ports/export.ts` (modificado)
- `src/infrastructure/tauri/export.ts` (modificado)
- `src/infrastructure/memory/export.ts` (modificado)
- `src/infrastructure/memory/export.test.ts` (modificado)
- `src/application/use-cases.ts` (modificado)
- `src/ui/store/export-slice.ts` (modificado)
- `src/ui/store/export-slice.test.ts` (modificado)
- `src/ui/store/workspace.ts` (modificado)
- `src/ui/screens/ExportDialog.tsx` (modificado)
- `src/ui/screens/Export.test.tsx` (modificado)

## Pruebas
- RED:
  - `npx vitest run src/domain/export.test.ts src/infrastructure/memory/export.test.ts src/ui/store/export-slice.test.ts src/ui/screens/Export.test.tsx`
  - Fallaron 9 tests (funciones y métodos no definidos, `originInfo` no presente, componentes de UI de origen no renderizados).
- GREEN:
  - `npx vitest run src/domain/export.test.ts src/infrastructure/memory/export.test.ts src/ui/store/export-slice.test.ts src/ui/screens/Export.test.tsx` → 4 passed, 31 passed.
  - `npx vitest run` (suite completa) → 61 test files passed (61), 438 passed (438).
- Lint y tipos:
  - `npm run lint` → 0 errors, 0 warnings.
  - `npm run typecheck` → 0 errors.

## Autorrevisión
- Todos los mensajes fijos del plan y spec se implementaron textualmente: aviso de privacidad, mensajes de fórmulas quitadas, casos sin fila (singular y plural), límite de observaciones y toast de éxito.
- La opción de origen solo se muestra cuando el proyecto tiene información de guion (`originInfo !== null`).
- La opción se deshabilita si `unavailable !== null`, mostrando claramente la causa al usuario.
- Los tipos e interfaces coinciden exactamente con el contrato DTO de la Task 5.

## Dudas o riesgos
- Ninguno. Listo para Task 7 (cierre de bloque y verificación manual).

## Revisión (la llena el controlador)
- Spec: Pendiente · Calidad: Pendiente
- Hallazgos y cómo se resolvieron
