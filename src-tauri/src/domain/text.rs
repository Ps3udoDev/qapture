use crate::error::{AppResult, validation};

pub fn required_text(label: &str, value: &str, max: usize) -> AppResult<String> {
    let v = value.trim();
    if v.is_empty() {
        return Err(validation(format!("«{label}» es obligatorio.")));
    }
    check_max(label, v, max)?;
    Ok(v.to_string())
}

pub fn optional_text(label: &str, value: &str, max: usize) -> AppResult<String> {
    let v = value.trim();
    check_max(label, v, max)?;
    Ok(v.to_string())
}

fn check_max(label: &str, value: &str, max: usize) -> AppResult<()> {
    if value.chars().count() > max {
        return Err(validation(format!("«{label}» supera {max} caracteres.")));
    }
    Ok(())
}

/// Clave estable para una columna: minúsculas ASCII, sin acentos, separada por `_`, máximo 40.
pub fn slugify(label: &str) -> String {
    let mut out = String::new();
    for ch in label.trim().to_lowercase().chars() {
        let c = match ch {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            other => other,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    let limited: String = out.trim_end_matches('_').chars().take(40).collect();
    let limited = limited.trim_end_matches('_');
    if limited.is_empty() {
        "campo".into()
    } else {
        limited.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_strips_accents_and_symbols() {
        assert_eq!(slugify("Fecha de ejecución"), "fecha_de_ejecucion");
        assert_eq!(slugify("  ¿Código HTTP?  "), "codigo_http");
        assert_eq!(slugify("Año / Ñandú"), "ano_nandu");
        assert_eq!(slugify("!!!"), "campo");
        assert_eq!(slugify(&"a".repeat(80)).len(), 40);
    }

    #[test]
    fn required_and_optional_text() {
        assert_eq!(required_text("Nombre", "  Hola ", 10).unwrap(), "Hola");
        assert!(required_text("Nombre", " ", 10).is_err());
        assert!(optional_text("Cliente", "abcdef", 5).is_err());
        assert_eq!(optional_text("Cliente", "", 5).unwrap(), "");
    }
}
