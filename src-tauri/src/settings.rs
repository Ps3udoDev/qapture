//! Ajustes persistidos de la app (F6): perfil, atajos, sistema y exportación.

use serde::{Deserialize, Serialize};

use crate::{
    domain::text::optional_text,
    error::{AppResult, validation},
};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub profile: Profile,
    pub shortcuts: Shortcuts,
    pub system: SystemPrefs,
    pub export: ExportPrefs,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub tester: String,
    pub role: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Shortcuts {
    pub toggle: String,
    pub capture: String,
    pub paste: String,
    pub new_case: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            toggle: "Ctrl+Alt+Q".into(),
            capture: "Ctrl+Alt+S".into(),
            paste: "Ctrl+Alt+V".into(),
            new_case: "Ctrl+Alt+N".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SystemPrefs {
    pub autostart: bool,
    pub start_hidden: bool,
    pub close_to_tray: bool,
    pub esc_hides: bool,
}

impl Default for SystemPrefs {
    fn default() -> Self {
        Self {
            autostart: false,
            start_hidden: true,
            close_to_tray: true,
            esc_hides: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExportPrefs {
    pub default_folder: String,
    pub image_max_width: u32,
    pub include_summary: bool,
}

impl Default for ExportPrefs {
    fn default() -> Self {
        Self {
            default_folder: String::new(),
            image_max_width: 940,
            include_summary: true,
        }
    }
}

/// Recorta los textos y devuelve el primer error. No valida atajos ni la carpeta de exportación.
pub fn validate(input: &Settings) -> AppResult<Settings> {
    let mut out = input.clone();
    out.profile.tester = optional_text("Nombre del tester", &input.profile.tester, 100)?;
    out.profile.role = optional_text("Cargo", &input.profile.role, 100)?;
    out.profile.email = optional_text("Correo", &input.profile.email, 100)?;
    if !out.profile.email.is_empty() && !is_valid_email(&out.profile.email) {
        return Err(validation("«Correo» no es válido."));
    }
    if !(400..=1600).contains(&out.export.image_max_width) {
        return Err(validation(
            "El ancho máximo de imagen debe estar entre 400 y 1600 px.",
        ));
    }
    out.export.default_folder = input.export.default_folder.trim().to_string();
    Ok(out)
}

/// Exactamente una `@` con texto antes y un punto después.
fn is_valid_email(email: &str) -> bool {
    let mut parts = email.split('@');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(local), Some(domain), None) => {
            !local.is_empty() && domain.contains('.') && !email.contains(char::is_whitespace)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(f: impl FnOnce(&mut Settings)) -> Settings {
        let mut s = Settings::default();
        f(&mut s);
        s
    }

    fn message(s: &Settings) -> String {
        validate(s).unwrap_err().user_message()
    }

    #[test]
    fn validate_trims_and_rejects_with_fixed_messages() {
        let ok = validate(&with(|s| s.profile.tester = "  Ana  ".into())).unwrap();
        assert_eq!(ok.profile.tester, "Ana");
        assert_eq!(
            message(&with(|s| s.profile.tester = "a".repeat(101))),
            "«Nombre del tester» supera 100 caracteres."
        );
        assert_eq!(
            message(&with(|s| s.profile.role = "a".repeat(101))),
            "«Cargo» supera 100 caracteres."
        );
        assert_eq!(
            message(&with(|s| s.profile.email = "ana@".into())),
            "«Correo» no es válido."
        );
        assert!(validate(&with(|s| s.profile.email = "".into())).is_ok());
        assert!(validate(&with(|s| s.profile.email = "ana@acme.com".into())).is_ok());
        for width in [399, 1601] {
            assert_eq!(
                message(&with(|s| s.export.image_max_width = width)),
                "El ancho máximo de imagen debe estar entre 400 y 1600 px."
            );
        }
        assert!(validate(&with(|s| s.export.image_max_width = 400)).is_ok());
        assert!(validate(&with(|s| s.export.image_max_width = 1600)).is_ok());
    }
}
