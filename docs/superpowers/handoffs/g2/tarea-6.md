# Tarea 6 · Puerto, adaptadores y estado (G2-T4, parte 1)

**Estado:** DONE
**Commits:** `28170d9` (`feat(g2): puerto y estado de archivos de prueba`).

## Qué se implementó
- **Dominio (`src/domain/fixtures.ts`)**:
  - Interfaz `Fixture` con campos `id`, `caseId`, `fileName`, `size`, `sha256`, `note`, `position`, `addedAt`.
  - Constantes del contrato:
    - `FIXTURE_EXTENSIONS = ["xlsx", "xls", "csv", "txt", "pdf", "json", "xml", "zip"] as const`
    - `MAX_FIXTURE_BYTES = 25 * 1024 * 1024` (25 MB)
    - `FIXTURE_ACCEPT = ".xlsx,.xls,.csv,.txt,.pdf,.json,.xml,.zip"`
    - `FIXTURE_TOO_BIG = "El archivo de prueba supera 25 MB."`
    - `FIXTURE_NOT_ALLOWED = "Ese tipo de archivo no se acepta como archivo de prueba."`
  - Función pura `fixtureProblem(name: string, size: number): string | null`: valida extensión y tamaño previo a la lectura de bytes en el navegador.
- **Puerto (`src/ports/repositories.ts`)**:
  - Interfaz `FixtureStore` con métodos:
    - `list(caseId: string): Promise<Fixture[]>`
    - `add(caseId: string, fileName: string, bytes: Uint8Array): Promise<Fixture>`
    - `remove(id: string): Promise<void>`
    - `reveal(id: string): Promise<void>`
    - `copyPath(id: string): Promise<void>`
  - Campo `fixtures: FixtureStore` incorporado a la interfaz `Repositories`.
- **Adaptador Tauri (`src/infrastructure/tauri/fixtures.ts`)**:
  - `createTauriFixtures` (y alias `tauriFixtures`):
    - `list`: invoca `"fixture_list"` pasando `{ caseId }`, validado con `z.array(fixtureSchema)`.
    - `add`: empaqueta `{ caseId, fileName, note: "" }` y el `Uint8Array` con `encodeFrame`, invoca `"fixture_add"`, validado con `fixtureSchema`.
    - `remove`: invoca `"fixture_remove"` pasando `{ id }`.
    - `reveal`: invoca `"fixture_reveal"` pasando `{ id }`.
    - `copyPath`: invoca `"fixture_copy_path"` pasando `{ id }`.
  - Integrado en `src/infrastructure/tauri/repositories.ts`.
- **Adaptador en memoria (`src/infrastructure/memory/fixtures.ts`)**:
  - `createMemoryFixtures` que implementa `MemoryFixtureStore`:
    - Valida con `fixtureProblem(fileName, bytes.length)`.
    - Aplica límite de hasta 50 archivos por caso (`"El caso ya tiene 50 archivos de prueba."`).
    - Valida nombres duplicados sin distinguir mayúsculas en el mismo caso (`"El caso ya tiene un archivo llamado «<nombre>»."`).
    - Asigna identificadores `fix-<seq>`, `size`, hash mock, `position` consecutiva y `addedAt`.
    - En `remove`, elimina y compacta posiciones a `0..n`.
    - Registros `revealed: string[]` y `copied: string[]` para inspección en pruebas unitarias y de integración.
    - Método auxiliar `removeForCase(caseId)` para limpieza en cascada al borrar un caso o un proyecto.
  - Integrado en `src/infrastructure/memory/repositories.ts` en `MemoryRepositories`, `projects.remove` y `cases.remove`.
- **Estado Zustand (`src/ui/store/fixture-slice.ts`)**:
  - `FixtureSlice` con estado `fixtures: Fixture[]` y `fixturesCaseId: string | null`.
  - `loadFixtures(caseId)`: actualiza `fixturesCaseId`, consulta al repositorio y descarta la respuesta si el caso seleccionado cambió durante la llamada asíncrona (previene race conditions).
  - `addFixtures(caseId, files)`: procesa archivo por archivo; si `fixtureProblem` detecta error emite un toast `"error"` y continúa con los demás; si es válido, obtiene los bytes mediante `arrayBuffer()` y llama a `repos.fixtures.add`; finalmente recarga los fixtures del caso.
  - `removeFixture(id)`: invoca eliminación en repositorio y recarga el caso activo.
  - `revealFixture(id)`: invoca revelado en repositorio.
  - `copyFixturePath(id)`: invoca copia de ruta y emite toast de éxito `"Ruta copiada."`.
  - Integrado en `src/ui/store/workspace.ts` dentro de `WorkspaceState` y `createWorkspaceStore`.

## Archivos
- `src/domain/fixtures.ts` (creado)
- `src/domain/fixtures.test.ts` (creado)
- `src/ports/repositories.ts` (modificado)
- `src/infrastructure/tauri/fixtures.ts` (creado)
- `src/infrastructure/tauri/repositories.ts` (modificado)
- `src/infrastructure/memory/fixtures.ts` (modificado)
- `src/infrastructure/memory/repositories.ts` (modificado)
- `src/ui/store/fixture-slice.ts` (creado)
- `src/ui/store/fixture-slice.test.ts` (creado)
- `src/ui/store/workspace.ts` (modificado)

## Pruebas
- RED:
  - `npx vitest run src/domain/fixtures.test.ts src/ui/store/fixture-slice.test.ts` falló inicialmente porque no existían los módulos `src/domain/fixtures` ni las funciones e interfaces de la tarea.
- GREEN:
  - `npx vitest run src/domain/fixtures.test.ts src/ui/store/fixture-slice.test.ts`: 2 suites pasadas, 12 pruebas pasadas.
  - `npx vitest run`: 60 suites de prueba pasadas, 420 pruebas pasadas en total.
  - `cargo test`: 568 pruebas pasadas (564 lib, 4 bridge), 0 fallos.
- Verificación de tipos y linter:
  - `npm run typecheck`: 0 errores de TypeScript (`tsc --noEmit`).
  - `npm run lint`: 0 advertencias de ESLint (`eslint . --max-warnings 0`).

## Autorrevisión
- Se verificó que `loadFixtures` ignore respuestas tardías si el usuario navegó a otro caso antes de que termine `repos.fixtures.list`.
- Se verificó que `addFixtures` no aborte ante archivos inválidos (como `.exe`), mostrando el toast correspondiente y continuando con los archivos válidos del lote.
- `MemoryFixtureStore` mantiene consistencia exacta con las reglas de negocio de Rust (extensiones permitidas, 25 MB máximo, 50 archivos por caso, nombres únicos ignorando mayúsculas/minúsculas y compactación de posiciones al eliminar).
- Se preservaron las plantillas predeterminadas `builtinTemplates` en el módulo de memoria para retrocompatibilidad completa con pruebas existentes.

## Dudas o riesgos
- Ninguna. Queda lista la base para que la Tarea 7 implemente la sección visual `FixturesSection.tsx` en `CaseDetail.tsx`.

## Revisión (la llena el controlador)
- Spec:
- Hallazgos y cómo se resolvieron
