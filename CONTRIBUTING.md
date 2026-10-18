# Contribuir a Qapture

Gracias por ayudar. Este documento explica cómo preparar el entorno, cómo trabajar una fase y cómo publicar una versión. Para entender el código, empieza por [docs/arquitectura.md](docs/arquitectura.md).

## Requisitos

- Windows 10/11 con Microsoft Edge WebView2.
- Node 22+ y pnpm.
- Rust estable con el toolchain MSVC y Visual Studio Build Tools.

```bash
pnpm install
pnpm tauri dev      # app en modo desarrollo
pnpm tauri build    # instaladores en src-tauri/target/release/bundle
```

Si tienes Qapture instalada y abierta, la instancia única entrega el control a esa y la app de desarrollo no se abre: ciérrala con **Salir** en la bandeja.

## Ramas, commits y fases

- Una rama por fase o tarea: `feat/fN-nombre-corto` (por ejemplo `feat/f7-cicd-release`), desde `main` actualizado. Se integra con pull request.
- Commits convencionales y en español: `feat(f7): …`, `fix(f7): …`, `docs(f7): …`, `chore(release): vX.Y.Z`.
- Cada fase tiene un **contrato** en [docs/superpowers/contracts/](docs/superpowers/contracts/README.md) (alcance, tasks, criterios y checklist de pruebas) y un **plan** en `docs/superpowers/plans/`. El diseño general está en [docs/superpowers/specs/](docs/superpowers/specs/2026-10-15-qapture-design.md). Los textos visibles de la app van en español.
- Las decisiones grandes se anotan como [ADR](docs/adr/).

## Verificación antes de abrir un PR

Rust, desde `src-tauri/`:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Front, desde la raíz:

```bash
pnpm lint
pnpm typecheck
pnpm test
pnpm build
pnpm check-version    # las 4 fuentes de versión coinciden
```

Todo debe terminar con cero warnings; CI ejecuta lo mismo en cada PR.

### Fixture de contrato

`src/infrastructure/tauri/__fixtures__/contracts.json` lo genera Rust y lo validan los esquemas `zod` ([ADR 0004](docs/adr/0004-tests-de-contrato.md)). Si cambias un DTO, regénéralo y revisa el diff:

```bash
cd src-tauri
UPDATE_CONTRACTS=1 cargo test contract
```

No lo edites a mano: CI falla si el fixture cambia al correr los tests.

### Comandos nuevos

Un comando nuevo se registra en `src-tauri/src/commands/names.rs`, en `generate_handler!` (`lib.rs`) y en `src-tauri/capabilities/main.json` (o `overlay.json`). Los tests comprueban las tres cosas.

## Publicar una versión

Las versiones son `X.Y.Z` numéricas, sin `-rc` (el MSI no acepta prereleases), y cada una mayor que la anterior. El tag es `v` + versión. `main` está protegida (todo entra por PR), así que el commit de versión va por PR y el tag se pone después, sobre `main`.

1. Desde `main` actualizado, en una rama de release y con el árbol limpio:

   ```bash
   git checkout main && git pull
   git checkout -b release/vX.Y.Z
   pnpm release X.Y.Z --no-tag
   ```

   (opcional `--since <ref>` para elegir desde qué tag o commit se arma el CHANGELOG). El script sincroniza la versión en `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` y `src-tauri/Cargo.lock`, agrega la sección al `CHANGELOG.md` y hace el commit `chore(release): vX.Y.Z`. Con `--no-tag` no crea el tag. **No hace push**; imprime los pasos siguientes. Rechaza una versión que no sea mayor que la actual.

2. Sube la rama y abre el PR a `main`:

   ```bash
   git push -u origin release/vX.Y.Z
   ```

   Fusiónalo con **merge commit** (no squash). Si se fusiona con squash, el tag va igual sobre el commit que queda en `main`, no sobre el de la rama.

3. Con el PR fusionado, crea el tag sobre `main` y súbelo:

   ```bash
   git checkout main && git pull
   pnpm check-version vX.Y.Z
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

   `pnpm check-version vX.Y.Z` confirma que `main` tiene la versión del tag antes de crearlo. El push del tag dispara `release.yml`, que vuelve a verificar el tag contra las versiones, corre las pruebas, compila y crea un **release en borrador** con `Qapture_X.Y.Z_x64-setup.exe`, el `.msi`, sus `.sig`, `latest.json` y `Qapture_X.Y.Z_portable.zip`.

4. Revisa el borrador en GitHub (artefactos, `latest.json` y notas) y pasa el [checklist manual](docs/release-checklist.md).

5. **Publica el borrador.** Las apps instaladas solo ven el release cuando está publicado: `releases/latest` ignora borradores y prereleases.

Sin `--no-tag`, `pnpm release X.Y.Z` también crea el tag local; sirve solo en un repo sin protección de `main`.

La firma del updater usa los secretos `TAURI_SIGNING_PRIVATE_KEY` y `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` de GitHub; la clave pública va en `tauri.conf.json`. Si el repo se mueve de dueño, se publica un release puente que cambie la URL del updater.

## Datos y claves: nunca al repositorio

El repo es público.

- Nunca versiones datos de clientes ni capturas reales. `.gitignore` bloquea `*.db`, `data/`, `evidence/` y `*.xlsx`; los fixtures y semillas usan datos ficticios.
- Nunca versiones claves: la clave privada del updater (`*.key`) vive solo en los secretos de GitHub y en tu gestor de contraseñas.
