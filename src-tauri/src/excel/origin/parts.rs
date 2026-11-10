//! Partes de un `.xlsx` (ZIP) en su orden original, para reescribirlo sin alterarlo.

use std::io::{Cursor, Read, Write};

use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

use crate::error::{AppError, AppResult, validation};

fn export_error(error: impl std::fmt::Display) -> AppError {
    AppError::Export(error.to_string())
}

pub const NOT_XLSX: &str = "El archivo no es un libro de Excel (.xlsx) válido.";

#[derive(Debug)]
struct Part {
    name: String,
    method: CompressionMethod,
    data: Vec<u8>,
}

/// Las partes de un libro, en el orden y con el método de compresión del original.
#[derive(Debug)]
pub struct XlsxParts {
    parts: Vec<Part>,
}

impl XlsxParts {
    pub fn read(bytes: &[u8]) -> AppResult<Self> {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| validation(NOT_XLSX))?;
        let mut parts = Vec::with_capacity(archive.len());
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).map_err(|_| validation(NOT_XLSX))?;
            let mut data = Vec::new();
            file.read_to_end(&mut data)
                .map_err(|_| validation(NOT_XLSX))?;
            parts.push(Part {
                name: file.name().map_err(|_| validation(NOT_XLSX))?.into_owned(),
                method: file.compression(),
                data,
            });
        }
        let parts = Self { parts };
        if parts.get("xl/workbook.xml").is_none() || parts.get("[Content_Types].xml").is_none() {
            return Err(validation(NOT_XLSX));
        }
        Ok(parts)
    }

    pub fn names(&self) -> Vec<&str> {
        self.parts.iter().map(|p| p.name.as_str()).collect()
    }

    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.parts
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.data.as_slice())
    }

    pub fn text(&self, name: &str) -> AppResult<String> {
        let data = self
            .get(name)
            .ok_or_else(|| AppError::Export(format!("falta la parte {name}")))?;
        String::from_utf8(data.to_vec())
            .map_err(|e| AppError::Export(format!("la parte {name} no es UTF-8: {e}")))
    }

    /// Reemplaza conservando posición y método; si no existe, agrega al final comprimida.
    pub fn set(&mut self, name: &str, data: Vec<u8>) {
        match self.parts.iter_mut().find(|p| p.name == name) {
            Some(part) => part.data = data,
            None => self.parts.push(Part {
                name: name.to_string(),
                method: CompressionMethod::Deflated,
                data,
            }),
        }
    }

    pub fn write(&self) -> AppResult<Vec<u8>> {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        for part in &self.parts {
            zip.start_file(
                part.name.as_str(),
                SimpleFileOptions::default().compression_method(part.method),
            )
            .map_err(export_error)?;
            zip.write_all(&part.data)?;
        }
        Ok(zip.finish().map_err(export_error)?.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use zip::{ZipWriter, write::SimpleFileOptions};

    use super::*;
    use crate::excel::guion_fixture;

    #[test]
    fn read_then_write_keeps_every_part_and_its_order() {
        let original = guion_fixture::build();
        let parts = XlsxParts::read(&original).unwrap();
        let again = XlsxParts::read(&parts.write().unwrap()).unwrap();
        assert_eq!(again.names(), parts.names());
        for name in parts.names() {
            assert_eq!(again.get(name), parts.get(name), "{name}");
        }
    }

    #[test]
    fn set_replaces_in_place_or_appends() {
        let mut parts = XlsxParts::read(&guion_fixture::build()).unwrap();
        let before = parts
            .names()
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>();
        parts.set("xl/workbook.xml", b"<x/>".to_vec());
        parts.set("xl/nueva.xml", b"<y/>".to_vec());
        let after = parts.names();
        let before: Vec<&str> = before.iter().map(String::as_str).collect();
        assert_eq!(&after[..before.len()], before.as_slice());
        assert_eq!(after.last(), Some(&"xl/nueva.xml"));
        assert_eq!(parts.text("xl/workbook.xml").unwrap(), "<x/>");
    }

    #[test]
    fn rejects_files_that_are_not_xlsx() {
        let err = XlsxParts::read(b"no es un zip").unwrap_err();
        assert_eq!(err.to_string(), NOT_XLSX);
    }

    #[test]
    fn rejects_a_zip_without_workbook() {
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("a.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"hola").unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        let err = XlsxParts::read(&bytes).unwrap_err();
        assert_eq!(err.to_string(), NOT_XLSX);
    }
}
