use serde::{Deserialize, Serialize};

use crate::error::{AppResult, validation};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Longtext,
    List,
    Date,
    Number,
    Boolean,
}

impl FieldType {
    pub fn parse(value: &str) -> AppResult<Self> {
        Ok(match value {
            "text" => Self::Text,
            "longtext" => Self::Longtext,
            "list" => Self::List,
            "date" => Self::Date,
            "number" => Self::Number,
            "boolean" => Self::Boolean,
            other => return Err(validation(format!("Tipo de campo desconocido: «{other}»."))),
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Longtext => "longtext",
            Self::List => "list",
            Self::Date => "date",
            Self::Number => "number",
            Self::Boolean => "boolean",
        }
    }

    pub fn is_textual(self) -> bool {
        matches!(self, Self::Text | Self::Longtext)
    }

    pub fn max_len(self) -> usize {
        match self {
            Self::Text => 500,
            Self::Longtext => 16000,
            Self::List => 100,
            Self::Date => 10,
            Self::Number => 40,
            Self::Boolean => 5,
        }
    }
}

pub struct FieldRule<'a> {
    pub label: &'a str,
    pub field_type: FieldType,
    pub options: &'a [String],
    pub required: bool,
}

/// Texto: se conserva tal cual (vacío si solo hay espacios). Resto: recortado;
/// los números con coma decimal simple (`1,5`) pasan a punto.
pub fn normalize_value(field_type: FieldType, raw: &str) -> String {
    if field_type.is_textual() {
        return if raw.trim().is_empty() {
            String::new()
        } else {
            raw.to_string()
        };
    }
    let v = raw.trim();
    if field_type == FieldType::Number && is_comma_decimal(v) {
        return v.replacen(',', ".", 1);
    }
    v.to_string()
}

fn is_comma_decimal(v: &str) -> bool {
    let body = v.strip_prefix(['+', '-']).unwrap_or(v);
    match body.split_once(',') {
        Some((int, frac)) => {
            !int.is_empty()
                && !frac.is_empty()
                && int.bytes().all(|b| b.is_ascii_digit())
                && frac.bytes().all(|b| b.is_ascii_digit())
        }
        None => false,
    }
}

fn is_number(v: &str) -> bool {
    v.bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.' | b'e' | b'E'))
        && v.parse::<f64>().map(f64::is_finite).unwrap_or(false)
}

pub fn is_iso_date(v: &str) -> bool {
    let b = v.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_ok()
}

pub fn validate_value(rule: &FieldRule, raw: &str) -> Result<(), String> {
    let v = normalize_value(rule.field_type, raw);
    let label = rule.label;
    if v.trim().is_empty() {
        return if rule.required {
            Err(format!("«{label}» es obligatorio."))
        } else {
            Ok(())
        };
    }
    let max = rule.field_type.max_len();
    if v.chars().count() > max {
        return Err(format!("«{label}» supera {max} caracteres."));
    }
    match rule.field_type {
        FieldType::Text | FieldType::Longtext => Ok(()),
        FieldType::List if rule.options.contains(&v) => Ok(()),
        FieldType::List => Err(format!(
            "«{label}» debe ser una de las opciones de la lista."
        )),
        FieldType::Date if is_iso_date(&v) => Ok(()),
        FieldType::Date => Err(format!("«{label}» debe ser una fecha válida (AAAA-MM-DD).")),
        FieldType::Number if is_number(&v) => Ok(()),
        FieldType::Number => Err(format!("«{label}» debe ser un número.")),
        FieldType::Boolean if v == "true" || v == "false" => Ok(()),
        FieldType::Boolean => Err(format!("«{label}» debe ser Sí o No.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule<'a>(field_type: FieldType, options: &'a [String], required: bool) -> FieldRule<'a> {
        FieldRule {
            label: "Campo",
            field_type,
            options,
            required,
        }
    }

    #[test]
    fn required_ignores_whitespace() {
        assert_eq!(
            validate_value(&rule(FieldType::Text, &[], true), "   "),
            Err("«Campo» es obligatorio.".into())
        );
        assert_eq!(
            validate_value(&rule(FieldType::Text, &[], false), ""),
            Ok(())
        );
    }

    #[test]
    fn list_values_must_be_options() {
        let options = vec!["Alta".to_string(), "Media".to_string()];
        assert!(validate_value(&rule(FieldType::List, &options, false), "Media").is_ok());
        assert!(validate_value(&rule(FieldType::List, &options, false), "Baja").is_err());
    }

    #[test]
    fn dates_must_exist() {
        assert!(validate_value(&rule(FieldType::Date, &[], false), "2026-02-28").is_ok());
        assert!(validate_value(&rule(FieldType::Date, &[], false), "2026-02-30").is_err());
        assert!(validate_value(&rule(FieldType::Date, &[], false), "2026-2-1").is_err());
    }

    #[test]
    fn numbers_accept_comma_decimal() {
        let r = rule(FieldType::Number, &[], false);
        assert!(validate_value(&r, "1,5").is_ok());
        assert_eq!(normalize_value(FieldType::Number, " 1,5 "), "1.5");
        assert!(validate_value(&r, "-3.25e2").is_ok());
        for bad in ["1.234,5", "0x10", "inf", "NaN", "abc"] {
            assert!(validate_value(&r, bad).is_err(), "{bad} debería rechazarse");
        }
    }

    #[test]
    fn booleans_are_true_or_false() {
        let r = rule(FieldType::Boolean, &[], false);
        assert!(validate_value(&r, "true").is_ok());
        assert!(validate_value(&r, "sí").is_err());
    }

    #[test]
    fn length_counts_unicode_chars() {
        let r = rule(FieldType::Text, &[], false);
        assert!(validate_value(&r, &"🚀".repeat(500)).is_ok());
        assert_eq!(
            validate_value(&r, &"🚀".repeat(501)),
            Err("«Campo» supera 500 caracteres.".into())
        );
    }

    #[test]
    fn longtext_keeps_line_breaks() {
        assert_eq!(
            normalize_value(FieldType::Longtext, "línea 1\nlínea 2\n"),
            "línea 1\nlínea 2\n"
        );
        assert_eq!(normalize_value(FieldType::Longtext, " \n "), "");
    }

    #[test]
    fn parse_round_trips() {
        for ty in ["text", "longtext", "list", "date", "number", "boolean"] {
            assert_eq!(FieldType::parse(ty).unwrap().as_str(), ty);
        }
        assert!(FieldType::parse("color").is_err());
    }
}
