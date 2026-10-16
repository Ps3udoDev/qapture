use rusqlite::{params, Connection};

use crate::{db::now, error::AppResult};

struct SeedField {
    key: &'static str,
    label: &'static str,
    ty: &'static str,
    options: &'static [&'static str],
    required: bool,
    clear: bool,
    width: i64,
}

fn field(key: &'static str, label: &'static str, ty: &'static str, width: i64) -> SeedField {
    SeedField { key, label, ty, options: &[], required: false, clear: false, width }
}

impl SeedField {
    fn required(mut self) -> Self {
        self.required = true;
        self
    }
    fn clear(mut self) -> Self {
        self.clear = true;
        self
    }
    fn options(mut self, options: &'static [&'static str]) -> Self {
        self.options = options;
        self
    }
}

struct SeedTemplate {
    id: &'static str,
    name: &'static str,
    test_type: &'static str,
    fields: Vec<SeedField>,
}

fn builtins() -> Vec<SeedTemplate> {
    vec![
        SeedTemplate {
            id: "tpl-funcional",
            name: "Funcional",
            test_type: "Funcional",
            fields: vec![
                field("entorno", "Entorno", "text", 21).required(),
                field("modulo", "Módulo", "text", 22),
                field("menu", "Menú", "text", 24),
                field("submenu", "Submenú", "text", 24),
                field("funcionalidad", "Funcionalidad", "text", 28),
                field("datos_entrada", "Datos de entrada", "longtext", 48),
                field("resultado_esperado", "Resultado esperado", "longtext", 44),
                field("resultado_obtenido", "Resultado obtenido", "longtext", 44).clear(),
                field("fecha_ejecucion", "Fecha de ejecución", "date", 15).clear(),
                field("responsable", "Responsable", "text", 24),
                field("observaciones", "Observaciones", "longtext", 40).clear(),
            ],
        },
        SeedTemplate {
            id: "tpl-regresion",
            name: "Regresión",
            test_type: "Regresión",
            fields: vec![
                field("entorno", "Entorno", "text", 21).required(),
                field("modulo", "Módulo", "text", 22),
                field("version_probada", "Versión probada", "text", 18),
                field("caso_origen", "Caso origen", "text", 20),
                field("resultado_esperado", "Resultado esperado", "longtext", 44),
                field("resultado_obtenido", "Resultado obtenido", "longtext", 44).clear(),
                field("fecha_ejecucion", "Fecha de ejecución", "date", 15).clear(),
                field("responsable", "Responsable", "text", 24),
                field("observaciones", "Observaciones", "longtext", 40).clear(),
            ],
        },
        SeedTemplate {
            id: "tpl-api",
            name: "API",
            test_type: "API",
            fields: vec![
                field("entorno", "Entorno", "text", 21).required(),
                field("endpoint", "Endpoint", "text", 40),
                field("metodo", "Método", "list", 12).options(&["GET", "POST", "PUT", "PATCH", "DELETE"]),
                field("request", "Request", "longtext", 48),
                field("response_esperado", "Response esperado", "longtext", 44),
                field("response_obtenido", "Response obtenido", "longtext", 44).clear(),
                field("codigo_http", "Código HTTP", "number", 12).clear(),
                field("fecha_ejecucion", "Fecha de ejecución", "date", 15).clear(),
                field("responsable", "Responsable", "text", 24),
                field("observaciones", "Observaciones", "longtext", 40).clear(),
            ],
        },
        SeedTemplate {
            id: "tpl-uat",
            name: "UAT",
            test_type: "UAT",
            fields: vec![
                field("entorno", "Entorno", "text", 21).required(),
                field("proceso_negocio", "Proceso de negocio", "text", 30),
                field("usuario_clave", "Usuario clave", "text", 24),
                field("pasos", "Pasos", "longtext", 48),
                field("resultado_esperado", "Resultado esperado", "longtext", 44),
                field("resultado_obtenido", "Resultado obtenido", "longtext", 44).clear(),
                field("aceptado", "Aceptado", "boolean", 12).clear(),
                field("fecha_ejecucion", "Fecha de ejecución", "date", 15).clear(),
                field("observaciones", "Observaciones", "longtext", 40).clear(),
            ],
        },
    ]
}

/// Inserta las plantillas incluidas que falten. Es idempotente: las versiones nuevas
/// de la app pueden agregar plantillas sin tocar las existentes.
pub fn ensure_builtin_templates(conn: &Connection) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    for template in builtins() {
        tx.execute(
            "INSERT OR IGNORE INTO template (id, name, test_type, builtin, created_at) VALUES (?1, ?2, ?3, 1, ?4)",
            params![template.id, template.name, template.test_type, now()],
        )?;
        for (position, f) in template.fields.iter().enumerate() {
            tx.execute(
                "INSERT OR IGNORE INTO template_field
                   (id, template_id, key, label, type, options, required, clear_on_duplicate, width, position)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    format!("{}-{}", template.id, f.key),
                    template.id,
                    f.key,
                    f.label,
                    f.ty,
                    serde_json::to_string(f.options)?,
                    f.required,
                    f.clear,
                    f.width,
                    position as i64
                ],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}
