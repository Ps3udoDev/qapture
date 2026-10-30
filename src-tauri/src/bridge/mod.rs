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

/// Cómo busca el puente a Qapture.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Attempt {
    /// Primer arranque: abre Qapture oculta si no está y espera hasta 10 s.
    Start,
    /// Se cortó el canal: espera hasta 10 s a que Qapture vuelva, sin abrirla. Si la cerró el
    /// instalador para actualizar, relanzarla bloquearía la instalación.
    Reconnect,
    /// Sin conexión, ante cada mensaje del agente: un solo intento, sin abrir Qapture.
    Retry,
}

impl Attempt {
    pub fn launches(self) -> bool {
        self == Attempt::Start
    }
    fn wait(self) -> Duration {
        match self {
            Attempt::Start | Attempt::Reconnect => Duration::from_secs(10),
            Attempt::Retry => Duration::ZERO,
        }
    }
    fn unavailable(self) -> String {
        match self {
            Attempt::Start => fallback::APP_UNAVAILABLE,
            Attempt::Reconnect | Attempt::Retry => fallback::APP_CLOSED,
        }
        .into()
    }
}

async fn authenticated(
    exe: &Path,
    data: &Path,
    attempt: Attempt,
) -> Result<BufReader<launch::Stream>, String> {
    let app = launch::app_path(exe);
    let stream = launch::connect_or_launch(
        &endpoint::channel_name(data),
        attempt.launches().then_some(app.as_path()),
        attempt.wait(),
    )
    .await
    .map_err(|_| attempt.unavailable())?;
    // Leer después del arranque: en la primera ejecución la app puede crear mcp.key.
    let key = endpoint::read_key(data).map_err(|_| attempt.unavailable())?;
    let mut stream = BufReader::new(stream);
    let hello = Hello {
        qapture: "hello".into(),
        proto: endpoint::CHANNEL_PROTO,
        key,
        bridge: env!("CARGO_PKG_VERSION").into(),
    };
    send_line(
        stream.get_mut(),
        &serde_json::to_string(&hello).map_err(|_| attempt.unavailable())?,
    )
    .await
    .map_err(|_| attempt.unavailable())?;
    let line = tokio::time::timeout(Duration::from_secs(2), read_line(&mut stream, 4096))
        .await
        .map_err(|_| attempt.unavailable())?
        .map_err(|_| attempt.unavailable())?
        .ok_or_else(|| attempt.unavailable())?;
    let reply: HelloReply = serde_json::from_str(&line).map_err(|_| attempt.unavailable())?;
    // Tras una actualización este puente ya es viejo: el agente debe volver a lanzarlo.
    if attempt != Attempt::Start && reply.app != env!("CARGO_PKG_VERSION") {
        return Err(fallback::app_updated(&reply.app));
    }
    if !reply.ok {
        return Err(reply.error.unwrap_or_else(|| attempt.unavailable()));
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

/// Repite el `initialize` del cliente en una conexión nueva.
async fn resume(
    exe: &Path,
    data: &Path,
    attempt: Attempt,
    relay: &mut relay::Relay,
) -> Result<BufReader<launch::Stream>, String> {
    let mut connection = authenticated(exe, data, attempt).await?;
    for line in relay.on_reconnect() {
        send_line(&mut connection, &line)
            .await
            .map_err(|_| attempt.unavailable())?;
    }
    Ok(connection)
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
        Ok(data) => match authenticated(&exe, data, Attempt::Start).await {
            Ok(stream) => Some(stream),
            Err(error) => {
                message = error;
                None
            }
        },
        Err(_) => None,
    };
    let mut connected = stream.is_some();
    // Mensaje del agente que llegó sin conexión y se reenvía al recuperarla.
    let mut carried: Option<String> = None;
    loop {
        let Some(connection) = stream.take() else {
            let Some(line) = stdin.next(MAX_LINE_BYTES).await? else {
                return Ok(());
            };
            if connected && let Ok(data) = &data {
                match resume(&exe, data, Attempt::Retry, &mut relay).await {
                    Ok(connection) => {
                        stream = Some(connection);
                        carried = Some(line);
                        continue;
                    }
                    Err(error) => message = error,
                }
            }
            if let Some(response) = fallback::respond(&line, &message) {
                send_line(&mut stdout, &response).await?;
            }
            continue;
        };
        connected = true;
        let (reader, mut writer) = tokio::io::split(connection);
        let mut reader = Lines::new(reader);
        let mut lost = false;
        if let Some(line) = carried.take() {
            let relay::Forward::Server(line) = relay.on_client_line(&line);
            lost = send_line(&mut writer, &line).await.is_err();
        }
        while !lost {
            tokio::select! {
                line = stdin.next(MAX_LINE_BYTES) => {
                    let Some(line) = line? else { return Ok(()); };
                    let relay::Forward::Server(line) = relay.on_client_line(&line);
                    lost = send_line(&mut writer, &line).await.is_err();
                }
                line = reader.next(MAX_LINE_BYTES) => {
                    match line { Ok(Some(line)) => if let Some(line) = relay.on_server_line(&line) { send_line(&mut stdout, &line).await?; }, _ => lost = true }
                }
            }
        }
        for error in relay.pending_errors() {
            send_line(&mut stdout, &error).await?;
        }
        drop(reader);
        drop(writer);
        let data = data.as_ref().map_err(|_| io::Error::other("sin datos"))?;
        match resume(&exe, data, Attempt::Reconnect, &mut relay).await {
            Ok(connection) => stream = Some(connection),
            Err(error) => message = error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_first_start_opens_qapture() {
        assert!(Attempt::Start.launches());
        assert!(!Attempt::Reconnect.launches());
        assert!(!Attempt::Retry.launches());
        assert_eq!(Attempt::Start.unavailable(), fallback::APP_UNAVAILABLE);
        assert_eq!(Attempt::Reconnect.unavailable(), fallback::APP_CLOSED);
    }
}
