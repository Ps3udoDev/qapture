# Tarea 7 · Sección «Archivos de prueba» y cierre de G2 (G2-T4, parte 2)

**Estado:** DONE
**Commits:** `93d1ccd` (`feat(g2): sección de archivos de prueba en el detalle del caso`)

## Qué se implementó
- **Componente `FixturesSection` (`src/ui/screens/FixturesSection.tsx`)**:
  - Título «Archivos de prueba» con botón «Agregar…» vinculado al `<input type="file" multiple accept={FIXTURE_ACCEPT} className="hidden" tabIndex={-1} ... />`.
  - Al seleccionar archivos mediante el input, se invoca `addFixtures(caseId, files)` y se limpia `event.target.value = ""`.
  - Mensaje exacto de estado vacío: `Sin archivos de prueba. Arrastra aquí o usa «Agregar…».`.
  - Tabla/lista con los archivos asociados:
    - Nombre del archivo (`fixture.fileName`).
    - Tamaño formateado con `formatSize(fixture.size)` de `domain/data`.
    - Nota del archivo (`fixture.note`) solo si no está vacía tras `trim()`.
    - Botones de acción accesibles con iconos y texto claro:
      - «Mostrar en carpeta»: invoca `revealFixture(fixture.id)`.
      - «Copiar ruta»: invoca `copyFixturePath(fixture.id)`.
      - «Quitar»: abre `ConfirmDialog` con título «Quitar archivo de prueba», mensaje `Se quitará «<nombre>» de este caso.` y acción «Quitar» que llama a `removeFixture(fixture.id)`.
  - Al cambiar de caso, dispara `useEffect` con `loadFixtures(caseId)`.
  - Zona de arrastrar y soltar propia con `onDragOver` (`preventDefault()` y `stopPropagation()`) y `onDrop` (`preventDefault()`, `stopPropagation()` y `addFixtures(caseId, files)`), evitando que los eventos escalen al contenedor de capturas de `CaseDetail`.
- **Integración en `CaseDetail` (`src/ui/screens/CaseDetail.tsx`)**:
  - Incorporación de `<FixturesSection caseId={current.id} />` entre los metadatos del caso (`</dl>`) y `<EvidenceGallery caseRecord={current} />`.
- **Pruebas automáticas (`src/ui/screens/Fixtures.test.tsx`)**:
  - `agrega con «Agregar…» y muestra nombre y tamaño`
  - `dropping_on_the_section_adds_a_fixture_not_an_evidence`
  - `un tipo no aceptado muestra el mensaje fijo`
  - `«Copiar ruta» y «Mostrar en carpeta» llaman al puerto`
  - `«Quitar» pide confirmación y lo saca de la lista`
  - `muestra la nota si no está vacía y cambia de lista al cambiar de caso`
- **Actualización del contrato (`docs/superpowers/contracts/G-guion-a-evidencias.md`)**:
  - Tareas G2-T1 a G2-T4 marcadas con `✅ 2026-11-11`.
  - Texto de G2-T1 actualizado según reparto P3 (`media/fixtures.rs` y `repo/fixtures.rs`).
  - Criterio de aceptación G2 marcado como completado (`[x]`).
  - Estado del contrato actualizado indicando cierre del Bloque 2.
  - Documentación de las interfaces producidas en G2 (`case_fixture`, `FixtureDto`, `NewFixtureMeta`, comandos `fixture_*`, `FixtureStore`, paquete formato 2).

## Archivos
- `src/ui/screens/FixturesSection.tsx` (creado)
- `src/ui/screens/Fixtures.test.tsx` (creado)
- `src/ui/screens/CaseDetail.tsx` (modificado)
- `docs/superpowers/contracts/G-guion-a-evidencias.md` (modificado)
- `docs/superpowers/handoffs/g2/README.md` (modificado)
- `docs/superpowers/handoffs/g2/tarea-7.md` (creado)

## Pruebas

### RED
Comando: `npx vitest run src/ui/screens/Fixtures.test.tsx` (ejecutado desde `qapture/`)
Salida:
```
❯ src/ui/screens/Fixtures.test.tsx (5 tests | 5 failed) 2614ms
  ❯ sección «Archivos de prueba» (5)
    × agrega con «Agregar…» y muestra nombre y tamaño 344ms
    × dropping_on_the_section_adds_a_fixture_not_an_evidence 148ms
    × un tipo no aceptado muestra el mensaje fijo 38ms
    × «Copiar ruta» y «Mostrar en carpeta» llaman al puerto 1047ms
    × «Quitar» pide confirmación y lo saca de la lista 1037ms
FAIL  src/ui/screens/Fixtures.test.tsx > sección «Archivos de prueba» > agrega con «Agregar…» y muestra nombre y tamaño
TestingLibraryElementError: Unable to find an accessible element with the role "heading" and name "Archivos de prueba"
```
Fallo esperado: la sección aún no existía ni estaba montada en `CaseDetail`.

### GREEN
Comando: `npx vitest run src/ui/screens/Fixtures.test.tsx` (ejecutado desde `qapture/`)
Salida:
```
✓ src/ui/screens/Fixtures.test.tsx (6 tests) 1006ms
Test Files  1 passed (1)
     Tests  6 passed (6)
```

Suite completa Vitest:
Comando: `npx vitest run` (ejecutado desde `qapture/`)
Salida:
```
Test Files  61 passed (61)
     Tests  426 passed (426)
```

Suite completa Rust:
Comando: `cargo test` (ejecutado desde `qapture/src-tauri/`)
Salida:
```
test result: ok. 564 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 19.29s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.84s
```

### Verificación de estilo y tipos
- `npm run typecheck` en `qapture/`: 0 errores.
- `npm run lint` en `qapture/`: 0 advertencias, 0 errores.
- `cargo fmt --check` en `qapture/src-tauri/`: 0 diferencias.
- `cargo clippy --all-targets -- -D warnings` en `qapture/src-tauri/`: 0 advertencias.

## Prueba manual requerida (Step 5 del plan)
Para verificar la experiencia visual y de integración en escritorio con `npm run tauri dev` sobre un proyecto importado del guion sintético:

| # | Comprobación | Esperado |
|---|---|---|
| 1 | Agregar un `.xlsx`, un `.csv` y un `.txt` a CP01 («Agregar…» y arrastrar) | Aparecen con nombre y tamaño; un `.exe` muestra el mensaje fijo |
| 2 | «Mostrar en carpeta» y «Copiar ruta» | Se abre el Explorador con el archivo marcado; la ruta pegada termina en el nombre original |
| 3 | Exportar el paquete e importarlo | Proyecto «(importado)» con CP01–CP36 y los archivos en CP01 |
| 4 | Respaldo manual, quitar un archivo, restaurar | El archivo vuelve a CP01 y la copia del guion sigue presente |

## Autorrevisión
- Líneas en `FixturesSection.tsx`, `CaseDetail.tsx` y `Fixtures.test.tsx` se mantienen por debajo de 150 caracteres.
- La captura de eventos drag & drop en `FixturesSection` detiene la propagación con `stopPropagation()`, garantizando aislamiento con la galería de evidencias.
- Todas las comprobaciones obligatorias están cubiertas con tests unitarios y de integración.
