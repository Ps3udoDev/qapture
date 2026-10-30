// Con un app manifest, cada comando propio necesita su permiso `allow-*` en alguna
// capability (capabilities/*.json): así el overlay de captura no puede llamar a los de datos.
#[allow(dead_code)]
mod names {
    include!("src/commands/names.rs");
}

fn main() {
    println!("cargo:rerun-if-changed=src/commands/names.rs");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(names::APP_COMMANDS)),
    )
    .expect("no se pudo preparar el build de Tauri");
}
