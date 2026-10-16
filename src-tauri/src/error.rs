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
        }
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::Validation(m) | Self::NotFound(m) => m.clone(),
            Self::Capture(_) => "No se pudo capturar la pantalla. Inténtalo nuevamente.".into(),
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
}
