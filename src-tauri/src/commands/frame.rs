use serde::de::DeserializeOwned;
use tauri::ipc::{InvokeBody, Request};

use crate::error::{AppResult, validation};

/// Cuerpo crudo de un `invoke` binario (las imágenes no viajan como JSON).
pub fn raw<'a>(request: &'a Request<'_>) -> AppResult<&'a [u8]> {
    match request.body() {
        InvokeBody::Raw(bytes) => Ok(bytes),
        InvokeBody::Json(_) => Err(validation("Se esperaba contenido binario.")),
    }
}

/// `[u32 LE largo][JSON][bytes]`: metadatos tipados más un PNG en una sola llamada.
pub fn split<T: DeserializeOwned>(body: &[u8]) -> AppResult<(T, &[u8])> {
    let bad = || validation("La solicitud binaria no tiene el formato esperado.");
    let header: [u8; 4] = body
        .get(..4)
        .ok_or_else(bad)?
        .try_into()
        .map_err(|_| bad())?;
    let len = u32::from_le_bytes(header) as usize;
    let end = 4usize.checked_add(len).ok_or_else(bad)?;
    let json = body.get(4..end).ok_or_else(bad)?;
    let meta = serde_json::from_slice(json).map_err(|e| {
        validation(format!(
            "La solicitud binaria no tiene el formato esperado: {e}"
        ))
    })?;
    Ok((meta, &body[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::NewEvidenceMeta;

    fn frame(meta: &str, payload: &[u8]) -> Vec<u8> {
        let mut out = (meta.len() as u32).to_le_bytes().to_vec();
        out.extend_from_slice(meta.as_bytes());
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn splits_meta_and_payload() {
        let body = frame(
            r#"{"caseId":"c","stagedPath":"p","description":"Descripción ✅","annotations":"{}"}"#,
            &[1, 2, 3],
        );
        let (meta, payload) = split::<NewEvidenceMeta>(&body).unwrap();
        assert_eq!(meta.description, "Descripción ✅");
        assert_eq!(payload, &[1, 2, 3]);
    }

    #[test]
    fn rejects_truncated_invalid_or_unknown_frames() {
        let mut too_long = frame("{}", b"");
        too_long[0] = 200;
        for body in [vec![], vec![1, 0], frame("{", b""), too_long] {
            assert!(split::<serde_json::Value>(&body).is_err());
        }
        let extra = frame(
            r#"{"caseId":"c","stagedPath":"p","description":"d","annotations":"{}","x":1}"#,
            b"",
        );
        assert!(split::<NewEvidenceMeta>(&extra).is_err());
    }
}
