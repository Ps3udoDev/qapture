# F7 — CI/CD, auto-update, release y documentación · `v1.0.0`

**Depende de:** F1–F6 (CI puede adelantarse desde F1) · **Plan:** [2026-10-17-f7-cicd-release](../plans/2026-10-17-f7-cicd-release.md)

## Objetivo
Que publicar sea automático: un tag `vX.Y.Z` produce el instalador NSIS, el MSI, el ZIP portable y la actualización firmada, y las apps instaladas se actualizan solas.

## Alcance
| Entra | Fuera |
|---|---|
| UC-24 `ci.yml` en cada PR/push y `release.yml` en cada tag | Firma Authenticode (requiere certificado; mejora futura) |
| UC-23 Updater: comprobar al iniciar y cada 6 h, diálogo, instalar y reiniciar | Canal beta separado (v1.1) |
| ZIP portable con `portable.flag` (datos junto al exe) | macOS / Linux |
| Documentación de usuario y de desarrollo | |
| Versionado semántico y CHANGELOG automático | |

## Tasks
| ID | Task | Estado |
|---|---|---|
| F7-T1 | `ci.yml` (windows-latest): pnpm install, typecheck, ESLint, Vitest, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `tauri build --no-bundle`, verificación de fixture de contrato sin cambios | ✅ |
| F7-T2 | Par de claves del updater (`pnpm tauri signer generate`); `TAURI_SIGNING_PRIVATE_KEY` y su contraseña en GitHub Secrets; pubkey en `tauri.conf.json` | ✅ |
| F7-T3 | `release.yml`: `tauri-apps/tauri-action` en tag `v*` → release en borrador con `.exe`, `.msi`, `latest.json`, `.sig` | ✅ |
| F7-T4 | Job portable: ZIP de `Qapture.exe` + `portable.flag` + LEEME; modo portable en Rust (directorio de datos junto al exe) | ✅ |
| F7-T5 | Updater en la app (plugins `updater` + `process`): UI en Acerca de y en la bandeja; deshabilitado en modo portable (avisa y enlaza) | ✅ |
| F7-T6 | `release-please` o script de versión: sincroniza `package.json`, `Cargo.toml` y `tauri.conf.json`, y genera el CHANGELOG | ✅ |
| F7-T7 | Docs: `docs/usuario/` (instalación, SmartScreen, uso, atajos, FAQ), `docs/arquitectura.md`, ADRs (D6, D7, D8, D12), `CONTRIBUTING.md` | ✅ |
| F7-T8 | README con badge de release y enlace de descarga | ✅ |

## Criterios de aceptación
- [ ] Un PR con un test roto queda bloqueado por CI.
  - Cubierto por: jobs de `ci.yml` (typecheck, ESLint, Vitest, fmt, clippy, cargo test, `tauri build --no-bundle`, fixture, versiones). Pendiente (manual): activar la protección de rama en `main` y comprobarla con un PR de prueba que rompa un test.
- [ ] Al hacer push de `v1.0.0` aparece un release en borrador con los 4 artefactos.
  - Cubierto por: `release.yml` (borrador, `.exe`, `.msi`, `latest.json`/`.sig`, ZIP portable) y `pnpm check-version v1.0.0`. Pendiente: se confirma con el primer release real (tag `v1.0.0` desde `main`).
- [ ] Una app instalada en v1.0.0 detecta v1.0.1 y se actualiza verificando la firma.
  - Cubierto por: tests Rust y de UI del updater (F7-T5). Pendiente: se confirma con el ensayo real `1.0.0` → `1.0.1` tras el primer release.

## Checklist de pruebas locales
| # | Prueba | Esperado | OK |
|---|---|---|---|
| 1 | Descargar el `.exe` desde Releases e instalar | Instala; el aviso de SmartScreen está documentado | ☐ |
| 2 | Publicar v1.0.1 con la app abierta | Aviso de actualización; instala y reinicia; los datos se conservan | ☐ |
| 3 | ZIP portable en una USB | Corre sin instalar; los datos quedan en `.\data` | ☐ |
| 4 | Seguir `docs/usuario` desde cero | Una persona nueva lo usa sin ayuda | ☐ |

## Punto de partida (lo que ya existe)

Los dos workflows llegaron en el PR #5 (`feat/ci-cd`), adelantados a la fase:

| Archivo | Ya hace | Le falta según el contrato y el spec |
|---|---|---|
| `.github/workflows/ci.yml` | En `windows-latest`, en push y PR a `main`, con dos jobs:<br>• front: `pnpm install`, `typecheck`, `test`, `build`;<br>• Rust: `fmt --check`, `clippy -D warnings`, `cargo test`, que ya compara el fixture de contrato | **ESLint** (el proyecto no tiene linter); **`tauri build --no-bundle`**; comprobar que ningún test reescribió el fixture de contrato; comprobar que las cuatro fuentes de versión coinciden; y **protección de rama** en `main`, sin la cual un PR roto se puede mergear igual |
| `.github/workflows/release.yml` | Con un tag `v*.*.*` o a mano: tests, `tauri-action` y release **publicado** con `.exe` y `.msi` (así salió `v0.4.0`) | Release en **borrador**; firma del updater (`latest.json` + `.sig`); **ZIP portable**; notas desde el CHANGELOG; comprobar que el tag coincide con la versión. Además, `workflow_dispatch` hace checkout de `main` aunque se le pase otro tag (bug) y el MSI sale `en-US` (el spec pide `es-ES`) |

No existen: clave del updater ni secretos (`gh secret list` está vacío), modo portable, updater en la app, CHANGELOG, `docs/usuario/`, `docs/arquitectura.md`, ADRs (`docs/adr/` está vacío) ni `CONTRIBUTING.md`. El README quedó en «v0.4.0».

## Notas de implementación (acordadas en el plan)

- **ESLint se agrega desde cero** con reglas mínimas: `@eslint/js` recommended, `typescript-eslint` recommended, y de `react-hooks` solo `rules-of-hooks` (error) y `exhaustive-deps` (warn → error en CI). No se activan las reglas del React Compiler de `react-hooks` 7: obligarían a refactorizar código estable sin beneficio para v1.
- **`tauri build --no-bundle --debug` en CI**, no en perfil release:
  - valida `tauri.conf.json`, las capabilities, el `beforeBuildCommand` y el embebido del front en unos 5 minutos en vez de 15;
  - el perfil release (LTO) lo compila `release.yml` en cada tag.
- **Firma del updater solo en release:** `createUpdaterArtifacts` va en `src-tauri/tauri.release.conf.json`, que `release.yml` pasa con `--config`. Así `pnpm tauri build` local no exige la clave privada.
- **Sin `tauri-plugin-process`:** el contrato lo nombra para reiniciar desde JS, pero aquí todo el updater es Rust, así que alcanza con `AppHandle::restart()`. En Windows el instalador NSIS en modo `passive` cierra la app y la vuelve a abrir.
- **Script propio en vez de `release-please`:**
  - un tag creado por el bot con `GITHUB_TOKEN` no dispara `release.yml`, así que exigiría un PAT;
  - `release-please` tampoco actualiza bien `Cargo.lock` en un proyecto que no es un workspace Rust;
  - `pnpm release <X.Y.Z>` hace el bump y el CHANGELOG, el commit y el tag quedan locales, y el push lo hace una persona (con `--no-tag`, el tag se crea sobre `main` después del PR).
- **CHANGELOG:** las versiones 0.1.0–0.6.0 se escriben a mano, un resumen por fase sacado de los contratos. Desde la siguiente se genera de los commits `feat`/`fix` (en el repo ya son convencionales: `feat(f6): …`).
- **Portable:**
  - **WebView2:** el WebView sigue guardando su caché y su `localStorage` (preferencias de UI) en `%LOCALAPPDATA%`, porque la ventana principal se crea desde la configuración. Los datos de Qapture (base, capturas, respaldos) sí van a `.\data`.
  - **Inicio con Windows:** se desactiva en portable, porque un exe en una USB no debe quedar en el registro.
  - **Single-instance:** la versión portable y la instalada comparten identificador. Si una ya está abierta, abrir la otra muestra la que estaba abierta. Va en la FAQ.
- **Ensayo antes de `v1.0.0`:** el contrato pide probar «v1.0.0 → v1.0.1». Como el pipeline mismo es lo que se prueba, el cierre propone:
  1. mergear con versión `1.0.0` y crear el tag desde `main`, lo que genera un borrador;
  2. revisar los artefactos y publicar;
  3. instalar;
  4. sacar `1.0.1` con un cambio menor y ver la actualización.

  Si algo falla, se borra el borrador y el tag y se corrige.
- **Fuera de F7:** E2E con WebdriverIO (spec §10) y `tauri-plugin-log` (spec §8) quedan para después de v1.

## Interfaces producidas

- **Comandos Tauri (Task 3):**
  - `check_update() -> Option<UpdateInfoDto>`: revisión manual; los errores de red usan el mensaje fijo de conexión;
  - `install_update() -> ()`: descarga, instala y reinicia; en portable responde con el mensaje de la versión portable.
  - `UpdateInfoDto { version, currentVersion, notes, date?, portable }` (camelCase).
- **Eventos:**
  - `update:available` (payload `UpdateInfoDto`): hay una versión nueva pendiente (no reabre una versión pospuesta);
  - `update:show` (payload `UpdateInfoDto`): clic en el ítem de la bandeja; abre el diálogo aunque la versión se haya pospuesto;
  - `update:progress { downloaded, total? }`: avance de la descarga.
- **`portable.flag`:** archivo vacío junto a `Qapture.exe`; con él, los datos van a `<carpeta del exe>\data` (base, `evidence\`, `backups\`, `evidence-sin-uso\`).
- **Scripts:** `pnpm lint` (`eslint . --max-warnings 0`), `pnpm release <X.Y.Z> [--since <ref>] [--no-tag]` (con `main` protegida se usa `--no-tag` y el tag se crea tras el merge; ver `CONTRIBUTING.md`) y `pnpm check-version [vX.Y.Z]`.
