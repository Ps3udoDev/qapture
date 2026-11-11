// Elementos consumidos en Task 3 (repo::fixtures) y Task 4 (transfer::package).
#![allow(dead_code)]

use std::fs;

use sha2::{Digest, Sha256};

use crate::{
    error::{AppResult, validation},
    media::storage::{self, EvidenceFiles},
};

pub const MAX_FIXTURE_BYTES: u64 = 25 * 1024 * 1024;
pub const EXTENSIONS: [&str; 8] = ["xlsx", "xls", "csv", "txt", "pdf", "json", "xml", "zip"];
pub const TOO_BIG: &str = "El archivo de prueba supera 25 MB.";
pub const NOT_ALLOWED: &str = "Ese tipo de archivo no se acepta como archivo de prueba.";
pub const BAD_NAME: &str = "El nombre del archivo de prueba no es válido.";

const DOS_DEVICE_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub fn clean_name(raw: &str) -> AppResult<String> {
    let trimmed = raw.trim();
    let char_count = trimmed.chars().count();
    if !(1..=200).contains(&char_count) {
        return Err(validation(BAD_NAME));
    }
    if trimmed.ends_with('.') || trimmed.ends_with(' ') {
        return Err(validation(BAD_NAME));
    }
    for c in trimmed.chars() {
        if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
            return Err(validation(BAD_NAME));
        }
    }
    let base = trimmed.split('.').next().unwrap_or(trimmed);
    if DOS_DEVICE_NAMES
        .iter()
        .any(|&d| d.eq_ignore_ascii_case(base))
    {
        return Err(validation(BAD_NAME));
    }

    let ext = match trimmed.rsplit_once('.') {
        Some((_, ext)) => ext,
        None => return Err(validation(NOT_ALLOWED)),
    };
    let ext_lower = ext.to_ascii_lowercase();
    if !EXTENSIONS.iter().any(|&e| e == ext_lower) {
        return Err(validation(NOT_ALLOWED));
    }

    Ok(trimmed.to_string())
}

pub fn check_size(len: u64) -> AppResult<()> {
    if len > MAX_FIXTURE_BYTES {
        Err(validation(TOO_BIG))
    } else {
        Ok(())
    }
}

pub fn fixture_rel(project_id: &str, fixture_id: &str, name: &str) -> String {
    format!("evidence/{project_id}/fixtures/{fixture_id}/{name}")
}

pub fn write(files: &EvidenceFiles, rel: &str, bytes: &[u8]) -> AppResult<()> {
    let abs = files.absolute(rel);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent)?;
    }
    storage::write_atomic(&abs, bytes)?;
    let mut p = fs::metadata(&abs)?.permissions();
    p.set_readonly(true);
    fs::set_permissions(&abs, p)?;
    Ok(())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_extensions_follow_d12() {
        assert_eq!(
            clean_name("  F13_valor_distinto.XLSX ").unwrap(),
            "F13_valor_distinto.XLSX"
        );
        for ok in [
            "a.csv", "b.txt", "c.pdf", "d.json", "e.xml", "f.zip", "g.xls",
        ] {
            assert!(clean_name(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "..",
            "a/b.txt",
            "a\\b.txt",
            "x:y.txt",
            "con.txt",
            "LPT1.csv",
            "a.txt.",
            "a\u{7}.txt",
        ] {
            assert_eq!(
                clean_name(bad).unwrap_err().to_string(),
                BAD_NAME,
                "{bad:?}"
            );
        }
        for bad in ["setup.exe", "script.ps1", "sin_extension"] {
            assert_eq!(
                clean_name(bad).unwrap_err().to_string(),
                NOT_ALLOWED,
                "{bad:?}"
            );
        }
        assert!(check_size(MAX_FIXTURE_BYTES).is_ok());
        assert_eq!(
            check_size(MAX_FIXTURE_BYTES + 1).unwrap_err().to_string(),
            TOO_BIG
        );
    }

    #[test]
    fn written_fixtures_are_read_only() {
        let tmp = crate::media::storage::TempDir::new();
        let files = tmp.files();
        let rel = fixture_rel("p1", "f1", "test.txt");
        write(&files, &rel, b"contenido").unwrap();
        let abs = files.absolute(&rel);
        assert!(std::fs::metadata(&abs).unwrap().permissions().readonly());
    }
}
