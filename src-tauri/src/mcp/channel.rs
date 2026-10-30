use super::{
    endpoint::{self, Hello},
    protocol::{self, Session},
};
use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader},
    sync::broadcast,
};

pub const MAX_LINE_BYTES: usize = 40 * 1024 * 1024;
pub type Handler = Arc<dyn Fn(Session, String) -> (Session, Vec<String>) + Send + Sync>;

pub struct Lines<R> {
    reader: BufReader<R>,
    pending: Vec<u8>,
}
impl<R: AsyncRead + Unpin> Lines<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            pending: Vec::new(),
        }
    }
    /// El buffer pertenece a la conexión: cancelar select! no pierde una línea parcial.
    pub async fn next(&mut self, limit: usize) -> io::Result<Option<String>> {
        read_buffered(&mut self.reader, &mut self.pending, limit).await
    }
}

/// Lee sin reservar memoria proporcional a un mensaje hostil sin salto de línea.
pub async fn read_line<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    limit: usize,
) -> io::Result<Option<String>> {
    let mut line = Vec::new();
    read_buffered(reader, &mut line, limit).await
}
async fn read_buffered<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    line: &mut Vec<u8>,
    limit: usize,
) -> io::Result<Option<String>> {
    loop {
        let buffer = reader.fill_buf().await?;
        if buffer.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "línea incompleta",
                ))
            };
        }
        let count = buffer
            .iter()
            .position(|b| *b == b'\n')
            .map(|i| i + 1)
            .unwrap_or(buffer.len());
        if line.len() + count > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "mensaje demasiado grande",
            ));
        }
        let done = buffer[count - 1] == b'\n';
        line.extend_from_slice(&buffer[..count]);
        reader.consume(count);
        if done {
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            return String::from_utf8(std::mem::take(line))
                .map(Some)
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "UTF-8 inválido"));
        }
    }
}

pub async fn send_line<W: AsyncWrite + Unpin>(writer: &mut W, line: &str) -> io::Result<()> {
    writer.write_all(line.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await
}

pub async fn serve<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    key: Arc<Mutex<String>>,
    handler: Handler,
    mut changed: broadcast::Receiver<()>,
    mut disconnect: broadcast::Receiver<()>,
) -> io::Result<()> {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = Lines::new(reader);
    let line = tokio::select! {
        _ = disconnect.recv() => return Ok(()),
        line = tokio::time::timeout(Duration::from_secs(2), reader.next(4096)) => line.map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "saludo vencido"))??,
    };
    let hello: Hello = serde_json::from_str(
        &line.ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "sin saludo"))?,
    )
    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "saludo inválido"))?;
    let reply = endpoint::check_hello(&hello, &key.lock().unwrap_or_else(|e| e.into_inner()));
    send_line(&mut writer, &serde_json::to_string(&reply)?).await?;
    if !reply.ok {
        return Ok(());
    }
    let mut session = Session::new();
    loop {
        tokio::select! {
            biased;
            _ = disconnect.recv() => return Ok(()),
            event = changed.recv() => {
                if matches!(event, Ok(()) | Err(broadcast::error::RecvError::Lagged(_))) {
                    send_line(&mut writer, &protocol::tools_list_changed()).await?;
                } else { return Ok(()); }
            }
            line = reader.next(MAX_LINE_BYTES) => {
                let Some(line) = line? else { return Ok(()); };
                let handler = handler.clone();
                let task = tokio::task::spawn_blocking(move || handler(session, line));
                let result = tokio::select! {
                    biased;
                    _ = disconnect.recv() => return Ok(()),
                    result = task => result.map_err(|_| io::Error::other("sesión interrumpida"))?,
                };
                session = result.0;
                for line in result.1 { send_line(&mut writer, &line).await?; }
            }
        }
    }
}

#[cfg(windows)]
fn pipe(name: &str, first: bool) -> io::Result<tokio::net::windows::named_pipe::NamedPipeServer> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{
            Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW,
            SECURITY_ATTRIBUTES,
        },
    };
    let sddl: Vec<u16> = "D:P(A;;GA;;;OW)(A;;GA;;;SY)\0".encode_utf16().collect();
    let mut descriptor = std::ptr::null_mut();
    // SAFETY: the UTF-16 string is terminated and output storage is valid.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let mut options = tokio::net::windows::named_pipe::ServerOptions::new();
    options
        .first_pipe_instance(first)
        .reject_remote_clients(true);
    // SAFETY: attributes and descriptor remain valid during the synchronous CreateNamedPipe call.
    let result = unsafe {
        options.create_with_security_attributes_raw(
            name,
            (&mut attributes as *mut SECURITY_ATTRIBUTES).cast(),
        )
    };
    // SAFETY: the descriptor was allocated by ConvertStringSecurityDescriptorToSecurityDescriptorW.
    unsafe {
        LocalFree(descriptor);
    }
    result
}

pub async fn listen(
    name: String,
    key: Arc<Mutex<String>>,
    handler: Handler,
    changed: broadcast::Sender<()>,
    disconnect: broadcast::Sender<()>,
    listening: Arc<std::sync::atomic::AtomicBool>,
) -> io::Result<()> {
    #[cfg(windows)]
    {
        let mut server = pipe(&name, true)?;
        listening.store(true, std::sync::atomic::Ordering::Release);
        loop {
            server.connect().await?;
            // Keep an instance alive while the next one is created: never relinquish the name.
            let next = pipe(&name, false)?;
            let (key, handler, changed, disconnect) = (
                key.clone(),
                handler.clone(),
                changed.subscribe(),
                disconnect.subscribe(),
            );
            tokio::spawn(async move {
                if let Err(e) = serve(server, key, handler, changed, disconnect).await {
                    eprintln!("Canal MCP: {e}");
                }
            });
            server = next;
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
        let path = std::path::PathBuf::from(&name);
        if let Ok(metadata) = std::fs::symlink_metadata(&path) {
            if !metadata.file_type().is_socket() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "el endpoint no es un socket",
                ));
            }
            if tokio::net::UnixStream::connect(&path).await.is_ok() {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    "el canal ya está en uso",
                ));
            }
            if metadata.uid() != endpoint::unix_uid() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "socket ajeno",
                ));
            }
            std::fs::remove_file(&path)?;
        }
        let listener = tokio::net::UnixListener::bind(&path)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        listening.store(true, std::sync::atomic::Ordering::Release);
        loop {
            let (stream, _) = listener.accept().await?;
            let (key, handler, changed, disconnect) = (
                key.clone(),
                handler.clone(),
                changed.subscribe(),
                disconnect.subscribe(),
            );
            tokio::spawn(async move {
                if let Err(e) = serve(stream, key, handler, changed, disconnect).await {
                    eprintln!("Canal MCP: {e}");
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn handler() -> Handler {
        let dir = crate::media::storage::TempDir::new();
        let state = crate::state::AppState::new(
            crate::db::open_in_memory().unwrap(),
            dir.files(),
            dir.path(),
            None,
            false,
            crate::state::GithubState::for_tests("http://127.0.0.1:9", "http://127.0.0.1:9"),
        );
        Arc::new(move |mut session, line| {
            let _keep_directory_alive = &dir;
            let replies = protocol::handle_line(
                &mut session,
                &state,
                &crate::mcp::host::RecordingHost::default(),
                &line,
            );
            (session, replies)
        })
    }
    fn test_connection() -> (
        tokio::io::DuplexStream,
        broadcast::Sender<()>,
        broadcast::Sender<()>,
        tokio::task::JoinHandle<io::Result<()>>,
    ) {
        let (server, client) = tokio::io::duplex(8192);
        let (changed, _) = broadcast::channel(8);
        let (disconnect, _) = broadcast::channel(8);
        let task = tokio::spawn(serve(
            server,
            Arc::new(Mutex::new("secret".into())),
            handler(),
            changed.subscribe(),
            disconnect.subscribe(),
        ));
        (client, changed, disconnect, task)
    }
    async fn hello(client: &mut tokio::io::DuplexStream) {
        send_line(
            client,
            &json!({"qapture":"hello","proto":1,"key":"secret","bridge":env!("CARGO_PKG_VERSION")})
                .to_string(),
        )
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn a_client_with_the_key_can_initialize_and_list_tools() {
        let (mut client, _changed, _disconnect, task) = test_connection();
        hello(&mut client).await;
        let mut reader = BufReader::new(client);
        assert!(
            serde_json::from_str::<Value>(&read_line(&mut reader, 8192).await.unwrap().unwrap())
                .unwrap()["ok"]
                .as_bool()
                .unwrap()
        );
        for (id, method) in [(1, "initialize"), (2, "tools/list")] {
            send_line(
                reader.get_mut(),
                &json!({"jsonrpc":"2.0","id":id,"method":method,"params":{}}).to_string(),
            )
            .await
            .unwrap();
            let response: Value = serde_json::from_str(
                &read_line(&mut reader, MAX_LINE_BYTES)
                    .await
                    .unwrap()
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(response["id"], id);
            if method == "tools/list" {
                assert_eq!(response["result"]["tools"].as_array().unwrap().len(), 1);
            }
        }
        drop(reader);
        task.await.unwrap().unwrap();
    }
    #[tokio::test]
    async fn a_client_without_hello_is_closed() {
        let (mut client, _changed, _disconnect, task) = test_connection();
        send_line(
            &mut client,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#,
        )
        .await
        .unwrap();
        assert!(task.await.unwrap().is_err());
        assert!(
            read_line(&mut BufReader::new(client), 8192)
                .await
                .unwrap()
                .is_none()
        );
    }
    #[tokio::test]
    async fn permissions_changed_sends_list_changed_to_open_sessions() {
        let (mut client, changed, _disconnect, task) = test_connection();
        hello(&mut client).await;
        let mut reader = BufReader::new(client);
        read_line(&mut reader, 8192).await.unwrap();
        changed.send(()).unwrap();
        let line = tokio::time::timeout(Duration::from_secs(2), read_line(&mut reader, 8192))
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&line).unwrap()["method"],
            "notifications/tools/list_changed"
        );
        drop(reader);
        task.await.unwrap().unwrap();
    }
    #[tokio::test]
    async fn disconnect_closes_an_authenticated_connection() {
        let (mut client, _changed, disconnect, task) = test_connection();
        hello(&mut client).await;
        let mut reader = BufReader::new(client);
        read_line(&mut reader, 8192).await.unwrap();
        disconnect.send(()).unwrap();
        task.await.unwrap().unwrap();
        assert!(read_line(&mut reader, 8192).await.unwrap().is_none());
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn named_pipe_acl_allows_the_current_user() {
        let name = format!(r"\\.\pipe\qapture-mcp-test-{}", uuid::Uuid::new_v4());
        let server = pipe(&name, true).unwrap();
        let client = tokio::net::windows::named_pipe::ClientOptions::new()
            .open(&name)
            .unwrap();
        server.connect().await.unwrap();
        drop(client);
    }
    #[tokio::test]
    async fn oversized_line_is_rejected_before_it_is_buffered() {
        let mut reader = BufReader::new(&b"123456789\n"[..]);
        assert_eq!(
            read_line(&mut reader, 5).await.unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
    #[tokio::test]
    async fn cancelling_a_read_preserves_the_partial_line() {
        let (mut writer, reader) = tokio::io::duplex(64);
        let mut reader = Lines::new(reader);
        writer.write_all(b"{\"id\":").await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), reader.next(64))
                .await
                .is_err()
        );
        writer.write_all(b"1}\n").await.unwrap();
        assert_eq!(
            reader.next(64).await.unwrap().as_deref(),
            Some("{\"id\":1}")
        );
    }
}
