# F7 — CI/CD, auto-update, release y documentación · `v1.0.0`

**Depende de:** F1–F6 (CI puede adelantarse desde F1) · **Plan:** se escribe al iniciar la fase.

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
| ID | Task |
|---|---|
| F7-T1 | `ci.yml` (windows-latest): pnpm install, typecheck, ESLint, Vitest, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `tauri build --no-bundle`, verificación de fixture de contrato sin cambios |
| F7-T2 | Par de claves del updater (`pnpm tauri signer generate`); `TAURI_SIGNING_PRIVATE_KEY` y su contraseña en GitHub Secrets; pubkey en `tauri.conf.json` |
| F7-T3 | `release.yml`: `tauri-apps/tauri-action` en tag `v*` → release en borrador con `.exe`, `.msi`, `latest.json`, `.sig` |
| F7-T4 | Job portable: ZIP de `Qapture.exe` + `portable.flag` + LEEME; modo portable en Rust (directorio de datos junto al exe) |
| F7-T5 | Updater en la app (plugins `updater` + `process`): UI en Acerca de y en la bandeja; deshabilitado en modo portable (avisa y enlaza) |
| F7-T6 | `release-please` o script de versión: sincroniza `package.json`, `Cargo.toml` y `tauri.conf.json`, y genera el CHANGELOG |
| F7-T7 | Docs: `docs/usuario/` (instalación, SmartScreen, uso, atajos, FAQ), `docs/arquitectura.md`, ADRs (D6, D7, D8, D12), `CONTRIBUTING.md` |
| F7-T8 | README con badge de release y enlace de descarga |

## Criterios de aceptación
- [ ] Un PR con un test roto queda bloqueado por CI.
- [ ] Al hacer push de `v1.0.0` aparece un release en borrador con los 4 artefactos.
- [ ] Una app instalada en v1.0.0 detecta v1.0.1 y se actualiza verificando la firma.

## Checklist de pruebas locales
| # | Prueba | Esperado | OK |
|---|---|---|---|
| 1 | Descargar el `.exe` desde Releases e instalar | Instala; el aviso de SmartScreen está documentado | ☐ |
| 2 | Publicar v1.0.1 con la app abierta | Aviso de actualización; instala y reinicia; los datos se conservan | ☐ |
| 3 | ZIP portable en una USB | Corre sin instalar; los datos quedan en `.\data` | ☐ |
| 4 | Seguir `docs/usuario` desde cero | Una persona nueva lo usa sin ayuda | ☐ |
