# Tarea 7 · Puerto y estado (G1-T7)

**Estado:** DONE
**Commits:** ver `git log` (`feat(g1): puerto, casos de uso y estado del import de guion`).

## Qué se implementó
- Dominio del guion con la forma del contrato Rust: perfil versionado, columnas y destinos discriminados, roles anulables, opciones explícitas, inspección, vista previa e input. Helpers de sugerencia de columnas/tipos/roles, perfil, estados distintos, letras de columna y comparación sin acentos.
- Puerto `GuionImporter` y adaptador Tauri para los cinco comandos, con validación de respuestas mediante los esquemas estrictos existentes. El caso de uso convierte errores de validación en errores del formulario.
- Repositorio en memoria con inspección sintética de 13 columnas, encabezados en la fila 15 y tres casos, códigos inválidos que bloquean el import, perfiles únicos sin distinguir mayúsculas Unicode, sugerencia por encabezados y gestión de tokens.
- Slice del asistente: apertura, vista previa, importación, cierre y borrado de perfiles. Todas las operaciones asíncronas usan `dataBusy` para excluir otras operaciones de datos. Importar selecciona y recarga el proyecto, cierra el asistente y muestra resumen y avisos. Los errores de validación conservan el asistente abierto.
- El contador de sesión evita que una respuesta pendiente vuelva a abrir o modificar un asistente cerrado. Un segundo import concurrente no llega al puerto ni reutiliza el token.

## Archivos
- `src/domain/guion.ts`, `guion.test.ts`; `src/ports/guion.ts`, `repositories.ts`.
- `src/infrastructure/tauri/guion.ts`, `guion.test.ts`, `repositories.ts`.
- `src/infrastructure/memory/guion.ts`, `guion.test.ts`, `repositories.ts`.
- `src/application/use-cases.ts`.
- `src/ui/store/guion-slice.ts`, `guion-slice.test.ts`, `workspace.ts`.

## Pruebas
Todos los comandos desde la raíz del proyecto.
- RED: `npx vitest run src/domain/guion.test.ts src/ui/store/guion-slice.test.ts` → 2 suites fallidas al resolver `domain/guion`, inexistente antes de implementar. Cubrían sugerencias, forma del perfil, import/cancelación/validación y exclusión con respuestas tardías.
- GREEN final: `npx vitest run src/domain/guion.test.ts src/ui/store/guion-slice.test.ts src/infrastructure/memory/guion.test.ts src/infrastructure/tauri/guion.test.ts src/ui/store/data-slice.test.ts` → 5 archivos, 23 pruebas correctas (11 nuevas y 12 de regresión del slice de datos).
- Las pruebas adicionales comprueban errores de código, token consumido, perfiles Unicode, sugerencia y borrado; los cinco nombres y argumentos IPC; y que un import pendiente excluye otro import.
- `npm run typecheck` → exit 0.
- `npm run lint` → exit 0.
- `git diff --check` → exit 0.

## Autorrevisión
- Sin dependencias nuevas, cambios Rust ni debilitar esquemas. El adaptador devuelve tipos compatibles con los DTOs estrictos y el dominio.
- El mapa `fold` coincide con el mapa de caracteres de Rust; `buildProfile` copia sus datos y evita modificar el mapa de estados por defecto o los destinos originales.
- Los códigos CP de la importación en memoria no avanzan la secuencia de nuevos casos de Qapture, que permanece en 1.
- El guard compartido `dataBusy` excluye también las operaciones ya existentes de importación, exportación y respaldos.

## Dudas o riesgos
- El adaptador en memoria es un doble de pruebas: simula conteos/casos y valida códigos y perfiles; no lee un Excel ni guarda la copia del origen o los valores de campos. Esas garantías están verificadas por las pruebas Rust de T1–T6.
- El endurecimiento ante llamadas IPC simultáneas que evadan el store sigue siendo el Minor de T6. El flujo de interfaz sí impide el segundo import y está cubierto por prueba.

## Correcciones (ronda 1)
- Minor señalado por revisión: los nombres de cuatro encabezados de ejemplo no coincidían con el fixture de G0. Se alinearon los 13 encabezados de memoria y de las pruebas con `guion_fixture::HEADERS` y el contrato; las 23 pruebas finales siguen verdes.

## Revisión (la llena el controlador)
