# Tarea 8 · Asistente «Importar guion de pruebas…» y cierre de G1 (G1-T8)

**Estado:** DONE. Código, verificación automática y prueba de escritorio completos (2026-11-11).
**Commits:** ver `git log` (`feat(g1): asistente de 3 pasos para importar el guion`).

## Qué se implementó
- Opción «Importar guion de pruebas…» en Datos, debajo de la importación de paquetes, y montaje del asistente cuando existe `guionWizard`.
- Tres pasos con selección de hoja y perfil, fila de encabezados editable, destinos por columna, tipos y roles, equivalencia de estados, carátula opcional, vista previa, errores y avisos, nombre del proyecto y guardado opcional de perfil.
- El perfil sugerido queda preseleccionado; «Siguiente» abre su vista previa sin mapear. Elegir otro perfil abre directamente su vista previa y selecciona la hoja de esa definición. Borrar un perfil requiere confirmación y actualiza el selector.
- «Importar» se deshabilita ante errores de vista previa, nombre vacío o una operación en curso. Los errores de validación conservan el asistente y se muestran en su formulario. Los controles y la navegación se bloquean durante las peticiones.
- La navegación a vista previa exige una respuesta nueva del mismo token; un error o una respuesta de una sesión anterior no reutiliza una vista previa vieja. Escape no cierra el asistente durante una operación ni cuando solo se está cancelando la confirmación de borrado.
- Los estados desconocidos se muestran como «Sin equivalencia (avisar)» hasta que el usuario los mapea. Mantener esa opción conserva la advertencia y el comportamiento de recuperación definido por Rust.
- Contrato actualizado: G1-T1 a T7 completadas, T8 completada en código y pruebas automáticas; criterio manual de G1 pendiente. Se documentan `project_source.definition` (P1) y la ausencia de renombrado de perfiles (P4).

## Archivos
- `src/ui/screens/ImportGuionDialog.tsx` y `ImportGuion.test.tsx`.
- `src/ui/screens/DataMenu.tsx`, `AppShell.tsx`.
- `docs/superpowers/contracts/G-guion-a-evidencias.md`.

## Pruebas
Todos los comandos desde la raíz del proyecto.
- RED: `npx vitest run src/ui/screens/ImportGuion.test.tsx` → 6 pruebas fallidas al buscar el menú «Importar guion de pruebas…», inexistente antes de la implementación. Las pruebas cubren los cinco recorridos exigidos y un error de validación.
- GREEN focalizado final: `npx vitest run src/ui/screens/ImportGuion.test.tsx` → 9 pruebas correctas. Se añadieron selección manual de encabezados, tipos/roles/estados y carátula; selección de un perfil de otra hoja y Escape en confirmación; bloqueo de navegación y cierre durante una petición pendiente.
- GREEN completo: `npx vitest run` → 58 archivos, 405 pruebas correctas, 0 fallidas.
- `npm run typecheck` → exit 0. El primer pase detectó una opción `exact` no admitida en `getByRole` dentro de las pruebas; se retiró manteniendo los nombres exactos y el pase final fue correcto.
- `npm run lint` → exit 0.
- `git diff --check` → exit 0.
- Verificación del controlador: `npm run build` → exit 0, 2189 módulos, Vite 7.21 s. Aviso no bloqueante: el chunk principal mide 505.92 kB, por encima del aviso de 500 kB; también aparecen mensajes de tiempos de Tailwind. No se cambia la división de bundles, ajena al alcance.
- Esta tarea no cambia código Rust. Verificación final coordinada por el controlador sobre la corrección `1609889`: `cargo test` → 543 pruebas de biblioteca y 4 de integración correctas, 4 ignoradas previas; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `git diff --check` → exit 0. No se ejecutó Cargo en paralelo desde T8.

## Autorrevisión
- Se reutilizan `Dialog`, `DialogFooter`, estilos y controles existentes, sin dependencias nuevas ni rediseño.
- Las pruebas usan `renderApp` de `ui/test-utils`, que es el helper real del repositorio; no existe el `renderWithStore` mencionado en el plan.
- La definición enviada al puerto mantiene `options` y `role` explícitos y conserva las celdas de carátula solo si el usuario las confirma.
- El asistente deja la persistencia y validación definitiva al puerto/Rust y respeta la exclusión mediante `dataBusy`.

## Dudas o riesgos
- **Prueba manual pendiente:** una persona debe ejecutar el recorrido descrito debajo. No se afirma que se haya probado la aplicación de escritorio ni que el recorrido tarde menos de un minuto.
- Minor señalado por el revisor: si no se sugirió una fila de encabezados, la inspección trae la carátula vacía y elegir la fila manualmente no vuelve a detectarla. El contrato de inspección de T6 no entrega combinaciones a la UI. Queda documentado para G4; no se amplían comandos en este bloque.

## Verificación manual pendiente
Con `npm run tauri dev`, elegir «Datos › Importar guion de pruebas…» e importar `%TEMP%\g0\guion_sintetico.xlsx`.

| Comprobación | Esperado | Resultado |
|---|---|---|
| Primera importación | 36 casos CP01–CP36, encabezados fila 15, 10 campos del guion y casilla Condicionado | ✅ La persona vio la fila 15 sugerida; el MCP confirma 36 casos CP01–CP36, 10 campos + `condicionado` (boolean) |
| Criticidad | Lista y valores normalizados | ✅ `list` con `Importante`/`Crítico`; los 36 valores dentro de la lista (MCP) |
| Tiempo | Menos de un minuto sin editar el archivo | ✅ Aproximadamente un minuto, según la persona |
| Guardar perfil e importar de nuevo | Perfil sugerido, vista previa sin volver a mapear | ✅ Tras marcar «Guardar como perfil», la siguiente importación preselecciona el perfil y pasa directo a la vista previa |

**Prueba de escritorio · 2026-11-11.** Una persona la hizo con `npm run tauri dev` y el guion sintético. El controlador revisó el resultado de dos formas: con el MCP local de Qapture (proyectos, campos y casos) y leyendo la base SQLite sin modificarla (`import_profile` y `project_source`). Se crearon cuatro proyectos, todos con 36 casos y encabezados en la fila 15. El perfil quedó guardado y vinculado al proyecto que lo creó.

- Al principio la segunda importación no sugería nada porque todavía no había ningún perfil guardado: «Guardar como perfil» es opcional. Con el perfil guardado, se comportó como se esperaba.
- **Minor para G4:** si se importa con un perfil ya guardado sin guardarlo de nuevo, `project_source.profile_id` queda `NULL`. Así lo define el plan: solo se enlaza el perfil creado con `save_profile_as`. G4 usa `definition`, que sí se guarda, así que no bloquea. Si hace falta saber qué perfil se usó, conviene enlazarlo en G4.

## Revisión (la llena el controlador)
- Spec: ✅ · Calidad: Aprobada. Revisión independiente del diff `1609889..f5a4d3e`, completada junto con la auditoría global.
- Sin Critical ni Important. Navegación, perfiles, confirmación de borrado, mapeo, errores y exclusión de operaciones verificados.
- Minor aceptado: seleccionar manualmente una fila de encabezados no recalcula la carátula cuando la inspección no la detectó; queda documentado para G4.
- Prueba manual solicitada a una persona mediante la conversación el 2026-11-11; sin resultado recibido. No se marca el criterio G1 como aceptado.
