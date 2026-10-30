pub mod fallback;
pub mod launch;
pub mod relay;

use crate::mcp::{
    channel::{Lines, MAX_LINE_BYTES, read_line, send_line},
    endpoint::{self, Hello, HelloReply},
};
use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::BufReader;

fn data_dir(exe: &Path) -> io::Result<PathBuf> {
    // AppImage copia el puente a <datos>/bin para sobrevivir al desmontaje del AppImage.
    let dir = exe
        .parent()
        .ok_or_else(|| io::Error::other("sin directorio"))?;
    if dir.join("app-path").is_file() && dir.file_name().is_some_and(|name| name == "bin") {
        return dir
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| io::Error::other("sin datos"));
    }
    let default = crate::system::portable::default_app_data_dir()
        .ok_or_else(|| io::Error::other("sin carpeta de datos"))?;
    crate::system::portable::resolve_data_dir(dir, &default)
        .map(|location| location.dir)
        .map_err(io::Error::other)
}

async fn authenticated(
    name: &str,
    exe: &Path,
    data: &Path,
) -> Result<BufReader<launch::Stream>, String> {
    let stream = launch::connect_or_launch(name, &launch::app_path(exe), Duration::from_secs(10))
        .await
        .map_err(|_| fallback::APP_UNAVAILABLE.to_string())?;
    // Leer después del arranque: en la primera ejecución la app puede crear mcp.key.
    let key = endpoint::read_key(data).map_err(|_| fallback::APP_UNAVAILABLE.to_string())?;
    let mut stream = BufReader::new(stream);
    let hello = Hello {
        qapture: "hello".into(),
        proto: endpoint::CHANNEL_PROTO,
        key,
        bridge: env!("CARGO_PKG_VERSION").into(),
    };
    send_line(
        stream.get_mut(),
        &serde_json::to_string(&hello).map_err(|_| fallback::APP_UNAVAILABLE.to_string())?,
    )
    .await
    .map_err(|_| fallback::APP_UNAVAILABLE.to_string())?;
    let line = tokio::time::timeout(Duration::from_secs(2), read_line(&mut stream, 4096))
        .await
        .map_err(|_| fallback::APP_UNAVAILABLE.to_string())?
        .map_err(|_| fallback::APP_UNAVAILABLE.to_string())?
        .ok_or_else(|| fallback::APP_UNAVAILABLE.to_string())?;
    let reply: HelloReply =
        serde_json::from_str(&line).map_err(|_| fallback::APP_UNAVAILABLE.to_string())?;
    if !reply.ok {
        return Err(reply
            .error
            .unwrap_or_else(|| fallback::APP_UNAVAILABLE.into()));
    }
    if reply.proto != endpoint::CHANNEL_PROTO || reply.app != env!("CARGO_PKG_VERSION") {
        return Err(format!(
            "qapture-mcp ({}) no es compatible con Qapture ({}). Reinstala Qapture para actualizar los dos.",
            env!("CARGO_PKG_VERSION"),
            reply.app
        ));
    }
    // Conservar las notificaciones que hayan llegado junto al saludo en una sola lectura.
    Ok(stream)
}

/// stdout tiene un único dueño y solo recibe líneas del protocolo.
pub async fn run() -> io::Result<()> {
    let exe = std::env::current_exe()?;
    let mut stdin = Lines::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let data = data_dir(&exe);
    let mut relay = relay::Relay::default();
    let mut message = fallback::APP_UNAVAILABLE.to_string();
    let mut stream = match &data {
        Ok(data) => match authenticated(&endpoint::channel_name(data), &exe, data).await {
            Ok(stream) => Some(stream),
            Err(error) => {
                message = error;
                None
            }
        },
        Err(_) => None,
    };
    loop {
        let Some(connection) = stream.take() else {
            while let Some(line) = stdin.next(MAX_LINE_BYTES).await? {
                if let Some(response) = fallback::respond(&line, &message) {
                    send_line(&mut stdout, &response).await?;
                }
            }
            return Ok(());
        };
        let (reader, mut writer) = tokio::io::split(connection);
        let mut reader = Lines::new(reader);
        loop {
            tokio::select! {
                line = stdin.next(MAX_LINE_BYTES) => {
                    let Some(line) = line? else { return Ok(()); };
                    let relay::Forward::Server(line) = relay.on_client_line(&line);
                    if send_line(&mut writer, &line).await.is_err() { break; }
                }
                line = reader.next(MAX_LINE_BYTES) => {
                    match line { Ok(Some(line)) => if let Some(line) = relay.on_server_line(&line) { send_line(&mut stdout, &line).await?; }, _ => break }
                }
            }
        }
        for error in relay.pending_errors() {
            send_line(&mut stdout, &error).await?;
        }
        drop(reader);
        drop(writer);
        let data = data.as_ref().map_err(|_| io::Error::other("sin datos"))?;
        match authenticated(&endpoint::channel_name(data), &exe, data).await {
            Ok(mut connection) => {
                let mut replay_ok = true;
                for line in relay.on_reconnect() {
                    if send_line(&mut connection, &line).await.is_err() {
                        replay_ok = false;
                        break;
                    }
                }
                if replay_ok {
                    stream = Some(connection);
                }
            }
            Err(error) => message = error,
        }
    }
}
