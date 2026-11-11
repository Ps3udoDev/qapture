# Revisión final · G1 Importar guion de pruebas

**Estado:** código y verificación automática aprobados; aceptación manual pendiente.
**Rama:** `feat/g1-importar-guion`.
**Diff revisado:** `7b9eab6..f5a4d3e`.
**Fecha:** 2026-11-11.
**Revisor:** subagente independiente del implementador, con revisión por tarea y auditoría integrada.

**Veredicto:** Spec ✅ en la implementación y contratos revisados · Calidad aprobada. No quedan hallazgos Critical ni Important abiertos. La rama es apta para revisión mediante un PR en borrador; este veredicto no acredita la prueba de escritorio ni el tiempo del recorrido manual.

## Alcance y método

Se contrastaron el [plan G1](../../plans/2026-11-10-g1-importar-guion.md), el [spec](../../specs/2026-11-10-guion-a-evidencias-design.md), el [contrato G](../../contracts/G-guion-a-evidencias.md), las decisiones de G0 y los diffs de las ocho tareas. La auditoría integrada siguió los datos desde el asistente hasta la persistencia y desde los casos importados hasta los editores existentes.

Se revisaron estáticamente las implementaciones, aserciones y reportes de ejecución. Los comandos de las suites completas fueron ejecutados por los implementadores y el controlador y constan en los handoffs; el revisor no repitió esas suites. El hallazgo Unicode de T3 se reprodujo independientemente con una tabla SQLite temporal en memoria.

## Veredicto por tarea

| Tarea | Commit principal | Veredicto | Observaciones |
|---|---|---|---|
| 1 · Lector | `806a186` | Spec ✅ · Calidad aprobada | Interfaces, fechas, hojas visibles, combinaciones y mensajes correctos; cobertura adicional opcional anotada abajo |
| 2 · Perfil | `ed09680` | Spec ✅ · Calidad aprobada | JSON versionado, validación, estados y plegado compartido; ambigüedad de claves normalizadas anotada abajo |
| 3 · Repositorios | `248d731` | Spec ✅ · Calidad aprobada | Important de nombres Unicode corregido; cascadas, snapshot y referencias a la copia comprobados |
| 4 · Vista previa | `fe9ea84` | Spec ✅ · Calidad aprobada | Important de Condiciones corregido; límites, errores, fechas, listas y carátula cubiertos |
| 5 · Persistencia | `361397d` | Spec ✅ · Calidad aprobada | Proyecto, perfil opcional, origen y copia se confirman o revierten juntos; regresión GitHub preservada |
| 6 · Comandos y contrato | `1afa730` | Spec ✅ · Calidad aprobada | Cinco comandos registrados, token, límites y esquemas estrictos; concurrencia IPC fuera del flujo UI anotada abajo |
| 7 · Puertos y estado | `a13fbe1` | Spec ✅ · Calidad aprobada | Contratos IPC y exclusión mediante `dataBusy` cubiertos; encabezados de memoria y pruebas alineados con G0 |
| 8 · Asistente | `f5a4d3e` | Spec ✅ en código y pruebas · Calidad aprobada | Recorridos, errores, perfiles, encabezados manuales y bloqueo durante peticiones cubiertos; validación humana pendiente |
| Correcciones integradas | `1609889` | Spec ✅ · Calidad aprobada | Listas importadas editables y creación/duplicado sin colisión de códigos |

## Hallazgos Important corregidos

1. **T3 · Nombres Unicode.** SQLite `NOCASE` permitía perfiles duplicados como `GUION ÁREA` y `guion área`. El repositorio compara y ordena con `to_lowercase`, conservando la distinción de acentos. La prueba demuestra el rechazo del duplicado, el orden y la validez de un nombre sin acento distinto. Ver [tarea 3](tarea-3.md), ronda 1.
2. **T4 · Tipo de Condiciones.** Anexar `Estado original: …` a una columna con rol Conditions y tipo lista o fecha dejaba un valor incompatible con el editor. El campo pasa a texto largo después de normalizar todas las filas y limpia sus opciones. La prueba cubre ambos tipos y una fecha serial posterior; el perfil original permanece intacto. Ver [tarea 4](tarea-4.md), ronda 1.
3. **Integración · Límites de listas.** La importación podía producir listas cuyos valores superaban 100 caracteres o cuyas opciones superaban 50, aceptadas al insertar pero rechazadas posteriormente por los editores. Esas listas pasan a texto largo al final de la normalización, con aviso y sin alterar el snapshot. Las regresiones importan y luego guardan tanto casos como columnas; también cubren listas predefinidas sin valores y el límite válido de 50 opciones de 100 caracteres.
4. **Integración · Códigos EJ importados.** Un guion con `EJ-001` y `EJ-002` dejaba bloqueada la creación o duplicación al conservar `next_sequence = 1`. La asignación de casos salta los códigos ocupados dentro de la transacción existente. El importador mantiene la secuencia inicial prevista por el plan; los casos nuevos reciben `EJ-003` y `EJ-004`, sin adquirir filas de origen. Los guiones CP01–CP36 siguen permitiendo `EJ-001`.

Las correcciones 3 y 4, sus decisiones y pruebas están en [correcciones de integración](correcciones-integracion.md).

## Minor aceptados y límites conocidos

- **T1 · Cobertura ampliable:** añadir pruebas específicas para un rango que empieza debajo de A1 y para encabezados ausentes o fuera de las primeras 200 filas. La implementación inspeccionada contempla estos casos; no se debilitaron las pruebas requeridas.
- **T2 · Claves de estado equivalentes:** el perfil admite claves diferentes que coinciden después de `fold`. Un perfil construido externamente podría asignarles destinos contradictorios. El asistente usa claves plegadas, pero no se añadió una guarda general de ambigüedad al perfil.
- **T6 · Consumo del token:** `with_guion` y `clear_guion` adquieren el candado por separado. El flujo de T7 excluye peticiones simultáneas mediante `dataBusy` y tiene una regresión de doble importación. Llamadas IPC concurrentes ajenas a ese flujo podrían reutilizar el pendiente o sustituirlo antes de limpiarlo.
- **T8 · Carátula y encabezados manuales:** si la inspección no sugirió encabezados, entrega la carátula vacía. Elegir la fila manualmente no vuelve a detectarla; corregir la fila tampoco recalcula esas celdas. El contrato actual no entrega las combinaciones a la UI. Queda documentado para ampliar la confirmación de carátula en G4, sin agregar comandos en este bloque.
- **Build:** Vite avisa que el chunk principal mide 505.92 kB, por encima del umbral de 500 kB. El build finaliza correctamente; la división de bundles no se modificó en G1.

## Evidencia automática final

| Comprobación | Resultado registrado |
|---|---|
| `npx vitest run` | 58 archivos, 405 pruebas correctas, 0 fallidas |
| Asistente `ImportGuion.test.tsx` | 9 pruebas correctas |
| `npm run typecheck` | Exit 0 |
| `npm run lint` | Exit 0 |
| `npm run build` | Exit 0; aviso de tamaño de chunk documentado |
| `cargo test` desde `src-tauri/` | Biblioteca: 543 correctas, 0 fallidas, 4 ignoradas previas; `tests/bridge.rs`: 4 correctas |
| `cargo fmt --check` | Exit 0 |
| `cargo clippy --all-targets -- -D warnings` | Exit 0 |
| `git diff --check` | Exit 0 |

La suite Rust final corresponde a `1609889`; T8 no modifica Rust. Los resultados completos constan en [tarea 8](tarea-8.md) y [correcciones de integración](correcciones-integracion.md). El mensaje previo `linker_messages` de MSVC al enlazar tests no produjo fallos; clippy terminó limpio.

## Persistencia y continuidad hacia G2/G4

- `project_source.definition` guarda el perfil usado independientemente de la biblioteca de perfiles. Borrarlo deja `profile_id` en NULL y conserva el snapshot.
- La copia del libro mantiene los bytes originales y su SHA-256; G1 no reescribe el Excel. Sus referencias impiden que la limpieza de evidencias la elimine.
- Los casos importados conservan su fila original. Crear o duplicar no agrega una fila de origen ficticia, por lo que G4 puede distinguir los casos que no se escriben.
- Las pruebas de rollback incluyen un fallo SQL posterior a la creación del perfil, proyecto y campos, y comprueban que no quedan esas filas ni la copia.
- La promoción de campos para mantenerlos editables modifica los campos del proyecto, conservando la definición de origen y los roles que utilizará G4.
- Paquetes de formato 2, respaldo de los nuevos archivos y exportación al formato de origen permanecen en G2/G4; no forman parte de la aprobación de código de G1.

## Revisión del controlador · 2026-11-11

- Suites repetidas de forma independiente sobre `5853fa7`: `cargo test` → 543 correctas, 4 ignoradas previas, y 4 de `tests/bridge.rs`; `cargo fmt --check` y `cargo clippy --all-targets -- -D warnings` limpios; `npx vitest run` → 405 correctas; `npm run typecheck` y `npm run lint` → exit 0.
- Contrastados contra el plan: dependencia única (`calamine`), migración 0007, snapshot `definition`, copia referenciada por `references`, token en `AppState`, `insert_in` transaccional y pruebas de rollback (perfil duplicado, errores de vista previa, fallo SQL después de crear el perfil).
- **Minor corregido:** `ImportGuionDialog.tsx` tenía líneas de hasta 705 caracteres (el resto de pantallas no pasa de ~250). Se reformateó y se extrajeron los manejadores (`chooseSheet`, `changeType`, `changeStatus`, `previewMapping`, `removeProfile`, `back`, `canImport`) sin cambiar el comportamiento; máximo 122 caracteres. `ImportGuion.test.tsx` → 9 correctas; typecheck y lint limpios. G4 ampliará este diálogo.

## Validación manual pendiente

**PENDIENTE:** una persona debe ejecutar el recorrido de [tarea 8](tarea-8.md) con `npm run tauri dev` y `%TEMP%\g0\guion_sintetico.xlsx`: comprobar 36 casos CP01–CP36, diez campos del guion más Condicionado, Criticidad como lista, duración inferior a un minuto y segunda importación con perfil sugerido sin remapear.

La comprobación fue solicitada por el controlador, pero no hay resultado humano registrado en esta revisión. El criterio de aceptación manual de G1 debe permanecer pendiente hasta incorporar esa evidencia.
