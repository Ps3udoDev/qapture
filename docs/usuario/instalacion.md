# Instalación

## Qué descargar

Entra a la página de [Releases](https://github.com/Ps3udoDev/qapture/releases) y baja los archivos de la versión más reciente:

| Archivo | Para quién |
|---|---|
| `Qapture_<versión>_x64-setup.exe` | **Recomendado.** Instalador normal. Se actualiza solo. |
| `Qapture_<versión>_x64_es-ES.msi` | Para el equipo de TI que despliega en varias computadoras. |
| `Qapture_<versión>_portable.zip` | No se instala: lo extraes y lo ejecutas (por ejemplo, desde una memoria USB). No se actualiza solo. |
| `Qapture_<versión>_amd64.AppImage` | **Linux, recomendado.** Funciona en casi cualquier distribución. Se actualiza solo. |
| `Qapture_<versión>_amd64.deb` | Linux Debian / Ubuntu / Mint. No se actualiza solo. |
| `Qapture-<versión>-1.x86_64.rpm` | Linux Fedora / openSUSE / RHEL. No se actualiza solo. |

## El aviso de SmartScreen

La primera vez que abras el instalador, Windows puede mostrar **«Windows protegió su PC»**. Es normal: el instalador de Qapture todavía no tiene firma de editor (Authenticode) en esta versión, y Windows desconfía de lo que no conoce. No significa que el archivo esté dañado.

Para continuar:

1. En la ventana azul, haz clic en **Más información**.
2. Aparece el botón **Ejecutar de todas formas**. Haz clic en él.
3. Sigue el instalador como cualquier otro.

Hazlo solo con archivos que hayas bajado de la página oficial de Releases.

## Instalación con MSI (equipos de TI)

El `.msi` sirve para desplegar Qapture en varias computadoras con las herramientas de TI. Las actualizaciones automáticas de la app usan el instalador NSIS (por usuario), no el MSI: en una instalación MSI, actualiza volviendo a desplegar el `.msi` de la versión nueva desde la herramienta de TI. Para una persona que instala Qapture en su propia computadora, se recomienda el `.exe` (NSIS).

## Versión portable

1. Extrae el ZIP en una carpeta donde puedas escribir, por ejemplo el Escritorio, Documentos o `D:\Qapture`. **No** lo extraigas en «Archivos de programa»: Qapture necesita guardar sus datos junto al programa y Windows no se lo permite ahí. Si lo intentas, verás el mensaje «No se puede escribir en <carpeta>. Copia Qapture a una carpeta con permisos de escritura (no en Archivos de programa).».
2. Ejecuta `Qapture.exe`.
3. No borres `portable.flag`. Ese archivo le dice a Qapture que trabaje en modo portable; sin él, la app usaría otra carpeta de datos y no verías tus proyectos.

Tus datos quedan en la carpeta `data`, junto a `Qapture.exe`. Para actualizar, descarga el ZIP nuevo y reemplaza `Qapture.exe`; `data` y `portable.flag` se conservan. El archivo `LEEME.txt` del ZIP repite estas indicaciones.

## Linux

Los paquetes se generan en Ubuntu 24.04, así que piden una distribución de esa época o más nueva (glibc 2.39: Ubuntu 24.04+, Debian 13+, Fedora 40+).

- **AppImage:** dale permiso de ejecución y ábrelo: `chmod +x Qapture_<versión>_amd64.AppImage && ./Qapture_<versión>_amd64.AppImage`. Si no arranca, instala FUSE 2 (`sudo apt install libfuse2t64` en Ubuntu 24.04).
- **.deb:** `sudo apt install ./Qapture_<versión>_amd64.deb`
- **.rpm:** `sudo dnf install ./Qapture-<versión>-1.x86_64.rpm`

Para ver el ícono en la bandeja en GNOME hace falta la extensión *AppIndicator* (Ubuntu ya la trae activada).

En sesiones **Wayland**, el sistema puede pedir permiso la primera vez que Qapture captura la pantalla, y algunos escritorios no dejan que una app registre atajos globales. Si el atajo no responde, inicia sesión con «Ubuntu en Xorg» o usa **Capturar pantalla** en el menú de la bandeja.

## Actualizaciones automáticas

La versión instalada busca actualizaciones al iniciar y luego cada 6 horas, aunque la app esté en la bandeja. Cuando hay una versión nueva:

- Se abre el aviso **Nueva versión disponible**: «Qapture <nueva> está disponible (tienes <actual>).».
- **Instalar y reiniciar** descarga la actualización, verifica su firma, la instala y reinicia Qapture. Tus datos se conservan.
- **Después** cierra el aviso, que no vuelve a abrirse solo para esa versión. Mientras tanto verás un punto en el botón de Ajustes y, en la bandeja, la opción **Instalar actualización <nueva>**, que vuelve a abrir el aviso.

También puedes buscar a mano: **Ajustes › Acerca de › Buscar actualizaciones**, o el menú de la bandeja (ícono de Qapture junto al reloj) › **Buscar actualizaciones**. Si ya tienes la última, verás «Qapture está al día (versión <actual>).».

Notas:

- Si hay una captura abierta sin guardar en el editor, Qapture no actualiza y te pide «Guarda o descarta la captura abierta antes de actualizar.», para no perderla.
- Sin internet, la revisión automática no muestra nada. La manual dice «No se pudo buscar actualizaciones. Revisa tu conexión a internet.».
- La versión portable no se instala sola: el aviso te lleva a la página de descargas con el botón **Descargar**, y en la bandeja la opción se llama **Descargar actualización <nueva>**.
- Una versión llega a las apps cuando se **publica** en Releases; los borradores no son visibles para la app.

## Desinstalar y dónde quedan tus datos

Desinstala desde **Configuración de Windows › Aplicaciones › Aplicaciones instaladas › Qapture**.

Desinstalar **no borra tus datos**. En la versión instalada viven en:

```
%APPDATA%\com.teams4soft.qapture
```

Pega esa ruta en el Explorador de archivos, o usa **Ajustes › Datos › Abrir carpeta de datos**. Ahí están la base de datos, las capturas (`evidence`) y los respaldos (`backups`). Si quieres borrar todo, elimina esa carpeta después de desinstalar. Antes, haz un respaldo o exporta lo que quieras conservar (ver [Uso](uso.md#paquetes-qapture-y-respaldos)).

En la versión portable, los datos están en la carpeta `data` junto a `Qapture.exe`.

En Linux los datos viven en `~/.local/share/com.teams4soft.qapture`. Para desinstalar: borra el `.AppImage`, o usa `sudo apt remove qapture` / `sudo dnf remove qapture`.
