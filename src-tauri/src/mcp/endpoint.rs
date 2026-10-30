//! Identidad del canal local y secreto compartido. Nunca registrar `Hello` ni la clave.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

pub const CHANNEL_PROTO: u32 = 1;
pub const INVALID_KEY: &str =
    "La clave del agente no es válida. Copia de nuevo la configuración desde Ajustes › Agentes.";

pub fn channel_name(data_dir: &Path) -> String {
    let hash = format!(
        "{:x}",
        Sha256::digest(data_dir.to_string_lossy().as_bytes())
    );
    let hash = &hash[..12];
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\qapture-mcp-{hash}")
    }
    #[cfg(unix)]
    {
        match std::env::var_os("XDG_RUNTIME_DIR") {
            Some(dir) => Path::new(&dir)
                .join(format!("qapture-mcp-{hash}.sock"))
                .to_string_lossy()
                .into_owned(),
            None => format!("/tmp/qapture-mcp-{}-{hash}.sock", unix_uid()),
        }
    }
}

#[cfg(unix)]
pub(crate) fn unix_uid() -> u32 {
    unsafe extern "C" {
        fn getuid() -> u32;
    }
    // SAFETY: getuid has no arguments and no failure case.
    unsafe { getuid() }
}

pub fn read_key(data_dir: &Path) -> io::Result<String> {
    let path = data_dir.join("mcp.key");
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::other("archivo de clave inválido"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if metadata.uid() != unix_uid() || metadata.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "permisos de clave inseguros",
            ));
        }
    }
    let mut key = String::new();
    fs::File::open(path)?.take(65).read_to_string(&mut key)?;
    if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(io::Error::other("formato de clave inválido"));
    }
    Ok(key)
}

fn random_key() -> io::Result<String> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(io::Error::other)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

pub fn load_or_create_key(data_dir: &Path) -> io::Result<String> {
    fs::create_dir_all(data_dir)?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(data_dir.join("mcp.key")) {
        Ok(mut file) => {
            let key = random_key()?;
            file.write_all(key.as_bytes())?;
            file.sync_all()?;
            Ok(key)
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => read_key(data_dir),
        Err(e) => Err(e),
    }
}

pub fn regenerate_key(data_dir: &Path) -> io::Result<String> {
    // El mutex del runtime serializa la rotación con los saludos y otras rotaciones.
    let key = random_key()?;
    let path = data_dir.join("mcp.key");
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    if path.exists() {
        read_key(data_dir)?;
    }
    let mut file = options.open(path)?;
    file.write_all(key.as_bytes())?;
    file.sync_all()?;
    Ok(key)
}

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut difference = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        difference |= (a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0)) as usize;
    }
    difference == 0
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hello {
    pub qapture: String,
    pub proto: u32,
    pub key: String,
    pub bridge: String,
}

#[derive(Serialize, Deserialize)]
pub struct HelloReply {
    pub ok: bool,
    pub proto: u32,
    pub app: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn check_hello(hello: &Hello, key: &str) -> HelloReply {
    let error = if hello.qapture != "hello"
        || !constant_time_eq(hello.key.as_bytes(), key.as_bytes())
    {
        Some(INVALID_KEY.into())
    } else if hello.proto != CHANNEL_PROTO || hello.bridge != env!("CARGO_PKG_VERSION") {
        Some(format!(
            "qapture-mcp ({}) no es compatible con Qapture ({}). Reinstala Qapture para actualizar los dos.",
            hello.bridge,
            env!("CARGO_PKG_VERSION")
        ))
    } else {
        None
    };
    HelloReply {
        ok: error.is_none(),
        proto: CHANNEL_PROTO,
        app: env!("CARGO_PKG_VERSION").into(),
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::storage::TempDir;
    #[test]
    fn channel_name_is_stable_and_differs_per_data_dir() {
        let a = Path::new("/absolute/data");
        assert_eq!(channel_name(a), channel_name(a));
        assert_ne!(channel_name(a), channel_name(Path::new("/portable/data")));
        #[cfg(windows)]
        assert!(channel_name(a).starts_with(r"\\.\pipe\qapture-mcp-"));
    }
    #[test]
    fn key_is_64_hex_and_reused() {
        let dir = TempDir::new();
        let key = load_or_create_key(dir.path()).unwrap();
        assert_eq!(key.len(), 64);
        assert!(key.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(key, load_or_create_key(dir.path()).unwrap());
    }
    #[test]
    fn regenerate_changes_the_key() {
        let dir = TempDir::new();
        let key = load_or_create_key(dir.path()).unwrap();
        let new = regenerate_key(dir.path()).unwrap();
        assert_ne!(key, new);
        assert_eq!(new, read_key(dir.path()).unwrap());
    }
    #[test]
    fn hello_ok_replies_proto_and_app_version() {
        let hello = Hello {
            qapture: "hello".into(),
            proto: 1,
            key: "secret".into(),
            bridge: env!("CARGO_PKG_VERSION").into(),
        };
        let reply = check_hello(&hello, "secret");
        assert!(reply.ok);
        assert_eq!(reply.proto, 1);
        assert_eq!(reply.app, env!("CARGO_PKG_VERSION"));
    }
    #[test]
    fn hello_with_wrong_key_is_rejected_with_the_fixed_message() {
        let hello = Hello {
            qapture: "hello".into(),
            proto: 1,
            key: "bad".into(),
            bridge: "1.1.9".into(),
        };
        assert_eq!(
            check_hello(&hello, "secret").error.as_deref(),
            Some(INVALID_KEY)
        );
    }
    #[test]
    fn hello_with_other_proto_reports_both_versions() {
        let hello = Hello {
            qapture: "hello".into(),
            proto: 2,
            key: "secret".into(),
            bridge: "1.1.9".into(),
        };
        assert_eq!(
            check_hello(&hello, "secret").error.unwrap(),
            format!(
                "qapture-mcp (1.1.9) no es compatible con Qapture ({}). Reinstala Qapture para actualizar los dos.",
                env!("CARGO_PKG_VERSION")
            )
        );
    }
    #[test]
    fn constant_time_comparison_rejects_prefixes_and_padding() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(!constant_time_eq(b"abc", b"abc\0"));
    }
    #[cfg(unix)]
    #[test]
    fn unix_key_file_is_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new();
        load_or_create_key(dir.path()).unwrap();
        assert_eq!(
            fs::metadata(dir.path().join("mcp.key"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
