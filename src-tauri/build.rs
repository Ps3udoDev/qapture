// Con un app manifest, cada comando propio necesita su permiso `allow-*` en alguna
// capability (capabilities/*.json): así el overlay de captura no puede llamar a los de datos.
#[allow(dead_code)]
mod names {
    include!("src/commands/names.rs");
}

fn main() {
    println!("cargo:rerun-if-changed=src/commands/names.rs");
    // Cargo test y el primer build del puente deben funcionar en un checkout limpio.
    // El empaquetador de Tauri lee la configuración original y exige el sidecar real;
    // beforeBuildCommand lo compila y copia antes de ese paso.
    let target = std::env::var("TARGET").expect("target Rust");
    let suffix = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    let sidecar = format!("binaries/qapture-mcp-{target}{suffix}");
    println!("cargo:rerun-if-changed={sidecar}");
    if !std::path::Path::new(&sidecar).is_file() {
        let mut config: serde_json::Value = std::env::var("TAURI_CONFIG")
            .ok()
            .and_then(|value| serde_json::from_str(&value).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        config["bundle"]["externalBin"] = serde_json::json!([]);
        // SAFETY: the build script is single-threaded and no threads have been started.
        unsafe {
            std::env::set_var("TAURI_CONFIG", config.to_string());
        }
    }
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(names::APP_COMMANDS)),
    )
    .expect("no se pudo preparar el build de Tauri");
}
