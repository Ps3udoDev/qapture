# Tarea 3 · Trasplante de una hoja con imágenes (G-T2)

**Estado:** DONE_WITH_CONCERNS (inquietud: `evidence_graft.rs` tiene 518 líneas con pruebas, más que las ~400 previstas)
**Commits:** ver `git log` de la rama (feat(g0): trasplantar una hoja con imágenes a un libro ajeno)

## Qué se implementó
`excel::origin::evidence_graft` con `pub fn graft_sheet(base, donor, name) -> AppResult<Vec<u8>>` y `pub(crate) fn next_free(names, prefix, suffix) -> u32`, siguiendo el algoritmo del plan paso a paso:

1. Lee ambos libros con `XlsxParts::read`. Valida el nombre (vacío, más de 31 caracteres, `[ ] : * ? / \`) y que no exista entre los `name="…"` de `<sheet …>` del base, comparando el texto desescapado (las cinco entidades).
2. Donante: exactamente una `xl/worksheets/sheet<n>.xml` (si no, `El libro de evidencias debe tener una sola hoja.`). Sus relaciones solo pueden ser `…/relationships/drawing` internas (si no, `El libro de evidencias tiene vínculos que no se pueden trasladar.`).
3. `N`, `D` y `M` con `next_free` sobre las partes del base (`xl/media/` sin extensión). Cada imagen se renombra `image{M}.{ext}`, `image{M+1}.{ext}`…
4. XML de la hoja: se quitan ` s="…"`, ` style="…"`, ` customFormat="…"` y ` tabSelected="…"` solo dentro de etiquetas (nunca del texto de celdas; ` s="` exige el espacio previo y el nombre exacto, así que `cols=`, `ss=` o `standalone=` no se tocan). Cada `<c … t="s"><v>i</v></c>` pasa a `t="inlineStr"` con `<is>{contenido del i-ésimo <si>}</is>` (`<si/>` → `<t></t>`).
5. Copia hoja, relaciones de la hoja (destino `../drawings/drawing{D}.xml`), dibujo, relaciones del dibujo (destinos `../media/…` renombrados en una sola pasada, sin encadenar renombres) e imágenes.
6. `xl/workbook.xml`: `<sheet name="{escapado}" sheetId="{máx+1}" r:id="rIdQ{k}"/>` justo antes de `</sheets>`.
7. `xl/_rels/workbook.xml.rels`: la `Relationship` de tipo worksheet antes de `</Relationships>`.
8. `[Content_Types].xml`: `Override` de la hoja y del dibujo, y `Default` por extensión nueva que no esté ya (`png` → `image/png`, `jpeg`/`jpg` → `image/jpeg`).
9. Ninguna otra parte del base cambia; devuelve `parts.write()`.

## Archivos
- `src-tauri/src/excel/origin/evidence_graft.rs` (nuevo: implementación + 7 pruebas del plan)
- `src-tauri/src/excel/origin/mod.rs` (`pub mod evidence_graft;`)

## Pruebas
- RED: `cargo test excel::origin::evidence_graft` -> 11 × `error[E0425]: cannot find function graft_sheet / next_free in this scope`; `could not compile qapture (lib test)`. Era el fallo esperado: las pruebas existían y las funciones aún no.
- GREEN: `cargo test excel::origin` -> `test result: ok. 12 passed; 0 failed` (5 de `parts`, que tiene una prueba más desde la corrección de la Tarea 2, + 7 de `evidence_graft`; el plan decía 11 porque contaba 4 de `parts`).
- Lint: `cargo fmt --check` limpio; `cargo clippy --all-targets -- -D warnings` limpio. La única línea extra en la salida de compilación es el `linker stdout: Creando biblioteca …` de MSVC, propio del entorno y previo a esta tarea.

## Autorrevisión
- Revisado el diff contra los pasos 1–9 del algoritmo: firmas, mensajes en español y valores (`rIdQ{k}`, tipos de contenido, rutas) tal como el plan.
- Las pruebas del plan están completas y con sus aserciones; solo cambió el formato de `cargo fmt`. Todas compilaron tal cual (incluido `&"x".repeat(32)` en el arreglo).
- Helper de prueba `donor(images)`: hoja «Donante», `A1` en negrita, `A2`, PNG de 300×200 con `write_png` (semilla distinta por imagen) desde la fila 3 cada 12 filas.
- Sin dependencias nuevas ni crates de XML/regex; todo con búsqueda de texto. `#[allow(dead_code)]` en `pub mod origin;` se mantiene (la producción aún no consume el módulo).
- Decisiones algo más estrictas que el plan (documentadas en el código):
  - El nombre repetido se compara sin distinguir mayúsculas (Excel no permite «hoja1» si existe «Hoja1»).
  - Las relaciones del dibujo solo pueden ser `…/relationships/image` internas, y la imagen debe existir en el donante; si no, el mismo error de vínculos (un gráfico o un vínculo en una imagen quedarían rotos).
  - Una extensión de imagen distinta de png/jpeg/jpg toma el `ContentType` que declara el `Default` del propio donante.
  - Si el XML del donante no tiene la forma esperada (índice de cadena compartida inexistente, etiqueta sin cerrar) o faltan `</sheets>`, `</Relationships>` o `</Types>` en el base, se devuelve `AppError::Export` con el detalle, sin tragar el error.

## Dudas o riesgos
- Tamaño: 518 líneas (≈ 330 de implementación con helpers pequeños y ≈ 190 de pruebas). No se partió el archivo; queda a criterio del controlador.
- El `<sheet>` insertado usa el prefijo `r:`; un libro cuyo `workbook.xml` declarara el espacio de nombres de relaciones con otro prefijo no lo entendería. Excel y `rust_xlsxwriter` siempre usan `r:`; la verificación manual (Tarea 4) lo confirmará con libros reales.
- La búsqueda de texto supone XML como el que escriben Excel y `rust_xlsxwriter` (atributos entre comillas dobles, sin comentarios ni CDATA en la hoja). El donante siempre lo genera Qapture, así que el supuesto se cumple.
