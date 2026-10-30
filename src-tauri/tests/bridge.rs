use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn bridge_logs_never_go_to_stdout() {
    let directory =
        std::env::temp_dir().join(format!("qapture-bridge-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let bridge = directory.join(if cfg!(windows) {
        "qapture-mcp.exe"
    } else {
        "qapture-mcp"
    });
    std::fs::copy(env!("CARGO_BIN_EXE_qapture-mcp"), &bridge).unwrap();
    std::fs::write(directory.join("portable.flag"), b"").unwrap();
    let mut process = Command::new(&bridge)
        .env("QAPTURE_MCP_NO_LAUNCH", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = process.stdin.take().unwrap();
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize"})
    )
    .unwrap();
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})
    )
    .unwrap();
    drop(stdin);
    let output = process.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    for line in lines {
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(line).unwrap()["jsonrpc"],
            "2.0"
        );
    }
    std::fs::remove_file(bridge).unwrap();
    std::fs::remove_file(directory.join("portable.flag")).unwrap();
    std::fs::remove_dir(directory).unwrap();
}

#[cfg(windows)]
#[tokio::test]
async fn bridge_reconnects_on_the_wire_without_repeating_initialize_to_the_client() {
    use serde_json::{Value, json};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::ServerOptions;
    let directory =
        std::env::temp_dir().join(format!("qapture-relay-test-{}", uuid::Uuid::new_v4()));
    let data = directory.join("data");
    std::fs::create_dir_all(&data).unwrap();
    let bridge = directory.join("qapture-mcp.exe");
    std::fs::copy(env!("CARGO_BIN_EXE_qapture-mcp"), &bridge).unwrap();
    std::fs::write(directory.join("portable.flag"), b"").unwrap();
    std::fs::write(data.join("mcp.key"), "a".repeat(64)).unwrap();
    let name = qapture_lib::mcp::endpoint::channel_name(&data);
    let server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(&name)
        .unwrap();
    let mut child = Command::new(&bridge)
        .env("QAPTURE_MCP_NO_LAUNCH", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{}", json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}})).unwrap();
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"qapture_status"}})
    )
    .unwrap();
    async fn take<R: tokio::io::AsyncRead + Unpin>(reader: &mut BufReader<R>) -> Value {
        let mut line = String::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            reader.read_line(&mut line),
        )
        .await
        .unwrap()
        .unwrap();
        serde_json::from_str(&line).unwrap()
    }
    async fn send<W: tokio::io::AsyncWrite + Unpin>(writer: &mut W, value: Value) {
        writer
            .write_all(format!("{value}\n").as_bytes())
            .await
            .unwrap();
    }
    server.connect().await.unwrap();
    let mut server = BufReader::new(server);
    assert_eq!(take(&mut server).await["qapture"], "hello");
    let hello = json!({"ok":true,"proto":1,"app":env!("CARGO_PKG_VERSION")});
    let changed = json!({"jsonrpc":"2.0","method":"notifications/tools/list_changed"});
    server
        .get_mut()
        .write_all(format!("{hello}\n{changed}\n").as_bytes())
        .await
        .unwrap();
    assert_eq!(take(&mut server).await["id"], 1);
    send(
        server.get_mut(),
        json!({"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18"}}),
    )
    .await;
    assert_eq!(take(&mut server).await["id"], 2);
    let second = ServerOptions::new().create(&name).unwrap();
    drop(server);
    second.connect().await.unwrap();
    let mut second = BufReader::new(second);
    assert_eq!(take(&mut second).await["qapture"], "hello");
    send(
        second.get_mut(),
        json!({"ok":true,"proto":1,"app":env!("CARGO_PKG_VERSION")}),
    )
    .await;
    assert_eq!(take(&mut second).await["id"], "qapture-replay");
    send(
        second.get_mut(),
        json!({"jsonrpc":"2.0","id":"qapture-replay","result":{}}),
    )
    .await;
    assert_eq!(
        take(&mut second).await["method"],
        "notifications/initialized"
    );
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","id":3,"method":"tools/list"})
    )
    .unwrap();
    assert_eq!(take(&mut second).await["id"], 3);
    send(
        second.get_mut(),
        json!({"jsonrpc":"2.0","id":3,"result":{"tools":[]}}),
    )
    .await;
    // Esperar que la tercera respuesta salga evita competir EOF stdin con el envío del servidor.
    let stdout = child.stdout.take().unwrap();
    let responses = tokio::task::spawn_blocking(move || {
        use std::io::BufRead;
        let mut lines = std::io::BufReader::new(stdout).lines();
        (0..4)
            .map(|_| serde_json::from_str::<Value>(&lines.next().unwrap().unwrap()).unwrap())
            .collect::<Vec<_>>()
    })
    .await
    .unwrap();
    assert_eq!(
        responses
            .iter()
            .filter_map(|v| v.get("id").cloned())
            .collect::<Vec<_>>(),
        vec![json!(1), json!(2), json!(3)]
    );
    assert_eq!(responses[0]["method"], "notifications/tools/list_changed");
    assert_eq!(responses[2]["result"]["isError"], true);
    drop(stdin);
    drop(second);
    assert!(
        tokio::task::spawn_blocking(move || child.wait().unwrap())
            .await
            .unwrap()
            .success()
    );
    std::fs::remove_file(bridge).unwrap();
    std::fs::remove_file(directory.join("portable.flag")).unwrap();
    std::fs::remove_file(data.join("mcp.key")).unwrap();
    std::fs::remove_dir(data).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
