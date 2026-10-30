use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

#[cfg(windows)]
pub type Stream = tokio::net::windows::named_pipe::NamedPipeClient;
#[cfg(unix)]
pub type Stream = tokio::net::UnixStream;

pub fn app_path(bridge_exe: &Path) -> PathBuf {
    let dir = bridge_exe.parent().unwrap_or(Path::new("."));
    if let Ok(path) = std::fs::read_to_string(dir.join("app-path")) {
        let path = PathBuf::from(path.trim());
        if path.is_absolute() {
            return path;
        }
    }
    dir.join(if cfg!(windows) {
        "qapture.exe"
    } else {
        "qapture"
    })
}

pub async fn connect(name: &str) -> io::Result<Stream> {
    #[cfg(windows)]
    {
        tokio::net::windows::named_pipe::ClientOptions::new().open(name)
    }
    #[cfg(unix)]
    {
        let stream = tokio::net::UnixStream::connect(name).await?;
        if stream.peer_cred()?.uid() != crate::mcp::endpoint::unix_uid() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "canal de otro usuario",
            ));
        }
        Ok(stream)
    }
}

pub async fn connect_or_launch(name: &str, app: &Path, wait: Duration) -> io::Result<Stream> {
    if let Ok(stream) = connect(name).await {
        return Ok(stream);
    }
    if std::env::var_os("QAPTURE_MCP_NO_LAUNCH").as_deref() == Some(std::ffi::OsStr::new("1")) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "inicio desactivado",
        ));
    }
    let mut command = Command::new(app);
    command
        .arg("--hidden")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = command.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    let deadline = tokio::time::Instant::now() + wait;
    loop {
        match connect(name).await {
            Ok(stream) => return Ok(stream),
            Err(e) if tokio::time::Instant::now() >= deadline => return Err(e),
            Err(_) => tokio::time::sleep(Duration::from_millis(250)).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::storage::TempDir;
    #[test]
    fn app_path_prefers_the_sibling_exe_and_reads_app_path_file() {
        let dir = TempDir::new();
        let exe = dir.path().join("qapture-mcp");
        assert_eq!(
            app_path(&exe),
            dir.path().join(if cfg!(windows) {
                "qapture.exe"
            } else {
                "qapture"
            })
        );
        let app = dir.path().join("Qapture.AppImage");
        std::fs::write(
            dir.path().join("app-path"),
            app.to_string_lossy().as_bytes(),
        )
        .unwrap();
        assert_eq!(app_path(&exe), app);
    }
}
