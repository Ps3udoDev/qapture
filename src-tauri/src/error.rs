use serde::{Serialize, Serializer, ser::SerializeStruct};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("base de datos: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("archivos: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("captura: {0}")]
    Capture(String),
    #[error("archivo en uso: {0}")]
    FileLocked(String),
    #[error("excel: {0}")]
    Export(String),
    #[error("base dañada: {0}")]
    Corrupt(String),
}

impl From<rust_xlsxwriter::XlsxError> for AppError {
    fn from(error: rust_xlsxwriter::XlsxError) -> Self {
        Self::Export(error.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn validation(message: impl Into<String>) -> AppError {
    AppError::Validation(message.into())
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation",
            Self::NotFound(_) => "not_found",
            Self::Db(_) => "db",
            Self::Io(_) => "io",
            Self::Json(_) => "internal",
            Self::Capture(_) => "capture",
            Self::FileLocked(_) => "file_locked",
            Self::Export(_) => "export",
            Self::Corrupt(_) => "corrupt",
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Validation(m) | Self::NotFound(m) => m.clone(),
            Self::Capture(_) => "No se pudo capturar la pantalla. Inténtalo nuevamente.".into(),
            Self::FileLocked(_) => "Cierra el archivo en Excel y vuelve a intentar.".into(),
            Self::Export(_) => "No se pudo generar el Excel. Inténtalo nuevamente.".into(),
            Self::Corrupt(_) => "La base de datos de Qapture está dañada.".into(),
            _ => "No se pudo completar la operación. Revisa el espacio disponible e inténtalo nuevamente.".into(),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AppError", 3)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.user_message())?;
        s.serialize_field("detail", &self.to_string())?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_errors_have_their_own_code_and_a_friendly_message() {
        let error = AppError::Capture("xcap: monitor desconectado".into());
        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["code"], "capture");
        assert_eq!(
            json["message"],
            "No se pudo capturar la pantalla. Inténtalo nuevamente."
        );
        assert_eq!(json["detail"], "captura: xcap: monitor desconectado");
    }

    #[test]
    fn a_locked_file_asks_to_close_it_in_excel() {
        let locked = serde_json::to_value(AppError::FileLocked("matriz.xlsx".into())).unwrap();
        assert_eq!(locked["code"], "file_locked");
        assert_eq!(
            locked["message"],
            "Cierra el archivo en Excel y vuelve a intentar."
        );
        assert_eq!(locked["detail"], "archivo en uso: matriz.xlsx");
    }

    #[test]
    fn excel_errors_have_their_own_code_and_a_friendly_message() {
        let export = serde_json::to_value(AppError::from(
            rust_xlsxwriter::XlsxError::SheetnameCannotBeBlank(String::new()),
        ))
        .unwrap();
        assert_eq!(export["code"], "export");
        assert_eq!(
            export["message"],
            "No se pudo generar el Excel. Inténtalo nuevamente."
        );
    }
}
