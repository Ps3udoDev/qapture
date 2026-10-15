# Qapture — Spec de diseño v1

> **Qapture** (QA + *capture*) · Ecosistema Teams4Soft
> Fecha: 2026-10-15 · Estado: **borrador para revisión**
> Reemplaza a: "Bitácora local" (`evidencias/app-local`, Node + HTML vanilla + CMD)

---

## 1. Contexto: qué existe hoy (Bitácora local v1)

| Pieza | Comportamiento actual | Problema para el largo plazo |
|---|---|---|
| `Abrir Bitacora.cmd` + `launcher.mjs` | Exige Node instalado, ejecuta `npm install`, lanza servidor HTTP `127.0.0.1:4318` y abre el navegador | No es distribuible al equipo; depende de Node y de un puerto libre |
| `server.mjs` | `GET/PUT /api/state` (estado completo, revisión optimista), `GET /api/export` | Guarda todo el estado en cada cambio |
| `store.mjs` | Un `matrices.json` (+`.bak`, escritura atómica), validación estricta, imágenes **base64 dentro del JSON**, tope 150 MB | No escala con muchas capturas |
| `export.mjs` | ExcelJS: hoja *Matriz* (14 columnas **fijas**), hoja *Evidencias* con imágenes y vínculos ida/vuelta, estados con color y lista desplegable | Sin carátula, sin branding Teams4Soft, columnas no configurables |
| `public/` | UI vanilla: proyectos, casos, Ctrl+V de capturas, duplicar ejecución, filtros, respaldo/importación JSON, plantilla vacía | Difícil de mantener/extender |

**Se conserva (comportamiento probado):** una fila por ejecución y entorno (`EJ-###`), duplicar ejecución, pegar captura con Ctrl+V, descripción obligatoria por captura, hoja de evidencias con vínculos "Ver evidencias / Volver a la matriz", seis estados con colores, respaldo e importación sin sobrescribir.

**Branding disponible** (extraído de `informes/*.docx`): logo *Teams4Soft · Inteligencia Híbrida*, banner de red de nodos y paleta:

| Token | Hex | Uso |
|---|---|---|
| `t4s-blue` | `#2B629E` | Encabezados, bandas, botones primarios |
| `t4s-lime` | `#BAC82D` | Acentos, KPI positivos, subrayados |
| `t4s-charcoal` | `#292B2D` | Texto principal |
| `t4s-cream` | `#F6F4F1` | Fondos de tabla / filas alternas |
| `t4s-gray` | `#596069` | Texto secundario, pies |
| Tipografía | Arial | Excel y documentos (segura en Office/WPS) |

## 2. Objetivo y criterios de éxito

**Objetivo:** que cualquier persona del equipo QA de Teams4Soft instale un `.exe`, tenga Qapture corriendo en segundo plano, lo invoque con un atajo, registre casos y capturas anotadas, y exporte un Excel con carátula Teams4Soft listo para entregar al cliente.

**Éxito significa:**
1. Instalación sin prerrequisitos (sin Node ni CMD), con instalador NSIS/MSI o versión portable.
2. Del atajo de captura a la evidencia anotada y guardada en menos de 15 segundos.
3. El Excel exportado abre sin advertencias en Excel y WPS e incluye carátula, matriz con columnas de la plantilla, evidencias y resumen.
4. Los datos de la v1 (`matrices.json` o respaldo JSON) se importan sin pérdida.
5. Las nuevas versiones llegan solas: el tag `vX.Y.Z` dispara el release y la app avisa e instala.

**No-objetivos v1:** servidor central, multiusuario en tiempo real, sincronización en la nube, macOS/Linux (el código queda portable, pero solo se publica Windows), edición del Excel de vuelta hacia la app.

## 3. Decisiones tomadas

| # | Decisión | Elección |
|---|---|---|
| D1 | Colaboración | **Local por equipo.** Se comparte el Excel o un paquete `.qapture` exportable/importable |
| D2 | Campos | **Plantillas de campos.** Biblioteca de plantillas por tipo de prueba; el proyecto copia la plantilla y puede ajustarla. Campos núcleo fijos |
| D3 | Carátula | Lo básico, más datos del proyecto, resumen de resultados y control de versiones |
| D4 | Distribución | **Repo público** `Ps3udoDev/qapture`, GitHub Releases, **auto-update firmado** |
| D5 | Atajos v1 | Mostrar/ocultar, pegar evidencia rápida, **captura integrada**, nuevo caso (todos configurables) |
| D6 | Arquitectura | **A+B:** React para todo lo visual (incluido el editor de anotaciones); Rust para captura, almacenamiento, archivos, Excel, bandeja, atajos y updater |
| D7 | Excel | Se genera en **Rust** con `rust_xlsxwriter` (robusto con cientos de imágenes, sin cargar el webview) |
| D8 | Persistencia | **SQLite** (`rusqlite`, bundled) para los datos estructurados. Imágenes como **archivos PNG** en el directorio de datos de la app |

## 4. Arquitectura

### 4.1 Vista de componentes

```mermaid
flowchart LR
  subgraph WebView["WebView2 — React 19 + TS"]
    UI["ui/<br/>pantallas, componentes,<br/>store Zustand"]
    APP["application/<br/>casos de uso"]
    DOM["domain/<br/>entidades, reglas,<br/>validación (TS puro)"]
    PORTS["ports/<br/>interfaces"]
    INFRA["infrastructure/tauri<br/>adaptadores invoke()"]
    ANN["ui/annotator<br/>Konva canvas"]
    UI --> APP --> DOM
    APP --> PORTS
    INFRA -. implementa .-> PORTS
    UI --> ANN
  end

  subgraph Core["Proceso Rust — Tauri 2"]
    CMD["commands/<br/>API tipada"]
    SVC["services/<br/>projects, cases, evidence,<br/>templates, export"]
    DB[("SQLite<br/>qapture.db")]
    FS[("evidence/<br/>PNG originales<br/>y renderizados")]
    XLSX["excel/<br/>rust_xlsxwriter"]
    CAP["capture/<br/>xcap + overlay"]
    SYS["tray · global-shortcut ·<br/>single-instance · autostart ·<br/>updater · clipboard"]
    CMD --> SVC --> DB
    SVC --> FS
    SVC --> XLSX
    CMD --> CAP
    CMD --> SYS
  end

  INFRA -- "invoke / events" --> CMD
  XLSX -- ".xlsx" --> OUT[/"Archivo elegido<br/>por el usuario"/]
  SYS -- "latest.json" --> GH[(GitHub Releases)]
```

### 4.2 Responsabilidades

| Capa | Hace | No hace |
|---|---|---|
| `domain/` (TS) | Entidades, validación de valores de campo según su tipo, cálculo de resumen, generación de `EJ-###`, plantillas por defecto | I/O, React, Tauri |
| `application/` (TS) | Orquesta casos de uso (`createCase`, `attachEvidence`, `exportProject`…) a través de puertos | Conocer SQLite o el canvas |
| `ports/` (TS) | `ProjectRepository`, `EvidenceStore`, `TemplateRepository`, `Exporter`, `ScreenCapture`, `ClipboardImage`, `SettingsStore`, `Updater` | — |
| `infrastructure/tauri` | Implementa los puertos con `invoke()`. También hay `infrastructure/memory` para tests | Lógica de negocio |
| `ui/` | Pantallas, formularios dinámicos según la plantilla, editor de anotaciones | Validación de negocio (la delega al dominio) |
| Rust `commands/` | API pública mínima, validada con `serde`. Errores tipados `AppError` serializados | Lógica de UI |
| Rust `services/` | Transacciones SQLite, ciclo de vida de archivos, export, migraciones, importación legacy | — |

**Reglas:**
- `ui → application → domain`; `infrastructure` implementa `ports`; `domain` no importa nada externo.
- Rust revalida todo lo que llega por `invoke` (defensa en profundidad), porque el dominio TS no es la única barrera.
- Los tipos compartidos TS↔Rust se generan con **`specta` / `tauri-specta`** para que no se desincronicen.

### 4.3 Estructura del repositorio

```
qapture/
├─ src/
│  ├─ domain/          entities.ts, fields.ts, status.ts, summary.ts, templates/defaults.ts
│  ├─ application/     use-cases/*.ts
│  ├─ ports/           *.ts
│  ├─ infrastructure/  tauri/*.ts, memory/*.ts, bindings.ts (generado)
│  ├─ ui/              app/, screens/, components/, annotator/, settings/, theme/
│  └─ assets/brand/    t4s-logo.png, t4s-network-banner.png
├─ src-tauri/
│  ├─ src/
│  │  ├─ commands/     projects.rs, cases.rs, evidence.rs, templates.rs, export.rs, capture.rs, settings.rs, legacy.rs
│  │  ├─ services/     …
│  │  ├─ db/           mod.rs, migrations/0001_init.sql …
│  │  ├─ excel/        cover.rs, matrix.rs, evidence_sheet.rs, summary.rs, styles.rs
│  │  ├─ capture/      screen.rs, overlay.rs
│  │  ├─ system/       tray.rs, shortcuts.rs, updater.rs
│  │  ├─ error.rs  lib.rs  main.rs
│  ├─ capabilities/    main.json, overlay.json (permisos mínimos por ventana)
│  └─ tauri.conf.json
├─ docs/               usuario/, arquitectura.md, adr/, superpowers/specs|plans
└─ .github/workflows/  ci.yml, release.yml
```

## 5. Modelo de datos

```mermaid
erDiagram
  TEMPLATE ||--o{ TEMPLATE_FIELD : define
  PROJECT ||--o{ PROJECT_FIELD : "copia de plantilla"
  PROJECT ||--o{ TEST_CASE : contiene
  PROJECT ||--o{ DOC_VERSION : "control de versiones"
  TEST_CASE ||--o{ EVIDENCE : tiene
  TEST_CASE ||--o{ FIELD_VALUE : valores
  PROJECT_FIELD ||--o{ FIELD_VALUE : tipa

  TEMPLATE {
    text id PK
    text name "Funcional, Regresión, API, UAT…"
    text test_type
    bool builtin
  }
  TEMPLATE_FIELD {
    text id PK
    text template_id FK
    text key
    text label
    text type "text|longtext|list|date|number|boolean"
    json options "para list"
    bool required
    int width "ancho Excel"
    int position
  }
  PROJECT {
    text id PK
    text name
    text client "p.ej. GMX"
    text test_type
    text release_version
    date start_date
    date end_date
    int next_sequence
    text source_template_id
    datetime created_at
    datetime updated_at
  }
  PROJECT_FIELD {
    text id PK
    text project_id FK
    text key
    text label
    text type
    json options
    bool required
    int width
    int position
    bool archived "no se borra si tiene datos"
  }
  TEST_CASE {
    text id PK
    text project_id FK
    text code "EJ-001"
    text title "núcleo"
    text status "núcleo"
    datetime created_at
    datetime updated_at
  }
  FIELD_VALUE {
    text case_id FK
    text field_id FK
    text value "texto normalizado por tipo"
  }
  EVIDENCE {
    text id PK
    text case_id FK
    int position
    text description
    text original_path "PNG inmutable"
    text rendered_path "PNG con anotaciones"
    json annotations "capa editable"
    int width
    int height
    datetime captured_at
  }
  DOC_VERSION {
    text id PK
    text project_id FK
    text version "1.0"
    date date
    text author
    text reviewer
    text approver
    text changes
  }
```

**Campos núcleo** (fijos en toda plantilla, no se pueden eliminar): `ID (EJ-###)`, `Caso de prueba`, `Estado`, `Evidencias` (calculado).

**Estados v1** (fijos, heredados de la v1): `No ejecutado`, `En ejecución`, `Por documentar`, `Aprobado`, `Fallido`, `Bloqueado`.

**Plantillas incluidas:**

| Plantilla | Campos (además de los núcleo) |
|---|---|
| **Funcional (por defecto)** | Entorno\*, Módulo, Menú, Submenú, Funcionalidad, Datos de entrada, Resultado esperado, Resultado obtenido, Fecha de ejecución, Responsable, Observaciones (equivale a la v1 + Observaciones) |
| Regresión | Entorno\*, Módulo, Versión probada, Caso origen, Resultado esperado, Resultado obtenido, Fecha, Responsable, Observaciones |
| API | Entorno\*, Endpoint, Método (lista), Request, Response esperado, Response obtenido, Código HTTP, Fecha, Responsable, Observaciones |
| UAT | Entorno\*, Proceso de negocio, Usuario clave, Pasos, Resultado esperado, Resultado obtenido, Aceptado (sí/no), Fecha, Observaciones |

**Reglas del modelo:**
- Un proyecto **copia** los campos de su plantilla. Editar la plantilla después no altera proyectos existentes.
- Un campo con datos no se elimina: se **archiva** (se oculta en UI y en Excel, sin perder valores). Un campo sin datos sí se elimina.
- Cambiar el tipo de un campo con datos solo se permite hacia `text` o `longtext`.

**Almacenamiento:** `%APPDATA%\com.teams4soft.qapture\` → `qapture.db` (modo WAL), `evidence/<project_id>/<evidence_id>.orig.png` y `.render.png`, `backups/` (copia automática diaria de la db, se conservan 7).

## 6. Casos de uso

### 6.1 Mapa general

```mermaid
flowchart TB
  QA(("👤 Tester QA"))
  LEAD(("👤 Líder QA"))
  SYS(("⚙️ Sistema / GitHub"))

  subgraph P["Proyectos y plantillas"]
    UC1["UC-01 Crear proyecto desde plantilla"]
    UC2["UC-02 Editar datos del proyecto<br/>(cliente, release, fechas)"]
    UC3["UC-03 Personalizar columnas del proyecto"]
    UC4["UC-04 Gestionar biblioteca de plantillas"]
    UC5["UC-05 Registrar versión del documento"]
  end
  subgraph C["Casos de prueba"]
    UC6["UC-06 Crear caso / ejecución"]
    UC7["UC-07 Editar caso y estado"]
    UC8["UC-08 Duplicar ejecución<br/>(otro entorno / reintento)"]
    UC9["UC-09 Buscar y filtrar"]
    UC10["UC-10 Eliminar caso"]
  end
  subgraph E["Evidencias"]
    UC11["UC-11 Capturar pantalla con atajo"]
    UC12["UC-12 Pegar evidencia del portapapeles"]
    UC13["UC-13 Arrastrar / elegir imagen"]
    UC14["UC-14 Anotar captura<br/>(flechas, números, texto, recuadros, desenfoque)"]
    UC15["UC-15 Reordenar / reasignar / eliminar evidencia"]
  end
  subgraph X["Salida e intercambio"]
    UC16["UC-16 Exportar Excel con carátula"]
    UC17["UC-17 Exportar plantilla vacía"]
    UC18["UC-18 Exportar / importar paquete .qapture"]
    UC19["UC-19 Importar datos de Bitácora v1"]
    UC20["UC-20 Respaldo y restauración"]
  end
  subgraph S["Sistema"]
    UC21["UC-21 Mostrar/ocultar con atajo y bandeja"]
    UC22["UC-22 Configurar atajos, tester, inicio con Windows"]
    UC23["UC-23 Buscar e instalar actualización"]
    UC24["UC-24 Publicar release automático"]
  end

  QA --> P & C & E & X & S
  LEAD --> UC4 & UC5 & UC16 & UC18
  SYS --> UC23 & UC24
  UC11 -. include .-> UC14
  UC12 -. include .-> UC14
  UC16 -. include .-> UC5
```

### 6.2 UC-11 Captura integrada con atajo (`Ctrl+Alt+S`)

```mermaid
sequenceDiagram
  actor T as Tester
  participant OS as Windows
  participant R as Rust (shortcuts/capture)
  participant O as Ventana overlay
  participant UI as React (main)
  participant DB as SQLite + evidence/

  T->>OS: Ctrl+Alt+S (app en segundo plano)
  OS->>R: evento global-shortcut
  R->>R: xcap: captura todos los monitores
  R->>O: abre overlay fullscreen sin bordes, con la captura congelada
  T->>O: arrastra región (Esc cancela, Enter = pantalla completa)
  O->>R: rect seleccionado
  R->>R: recorta y guarda PNG temporal
  R->>UI: muestra ventana principal + evento evidence:captured(tmp)
  UI->>T: Editor de anotaciones con caso activo preseleccionado
  T->>UI: anota, elige caso, escribe descripción, Guardar
  UI->>R: save_evidence(case_id, tmp, annotations, render_png, descripción)
  R->>DB: mueve original, escribe render, inserta EVIDENCE (transacción)
  R-->>UI: Evidence
  UI-->>T: toast "Evidencia 3 agregada a EJ-012"
```

### 6.3 UC-12 Pegar evidencia rápida (`Ctrl+Alt+V`, tras `Win+Shift+S`)

```mermaid
sequenceDiagram
  actor T as Tester
  participant R as Rust
  participant UI as React
  T->>R: Ctrl+Alt+V
  R->>R: lee imagen del portapapeles (plugin clipboard)
  alt hay imagen
    R->>UI: show + evidence:captured(tmp)
    UI->>T: editor de anotaciones (mismo flujo que UC-11)
  else no hay imagen
    R->>T: notificación "El portapapeles no contiene una imagen"
  end
```

Dentro de la app, `Ctrl+V` sigue funcionando igual que en la v1 (atajo local, no global).

### 6.4 UC-14 Editor de anotaciones

```mermaid
stateDiagram-v2
  [*] --> Abierto: captura / pegado / evidencia existente
  Abierto --> Dibujando: elige herramienta
  state Dibujando {
    [*] --> Flecha
    Flecha --> Numero
    Numero --> Texto
    Texto --> Recuadro
    Recuadro --> Resaltado
    Resaltado --> Desenfoque
  }
  Dibujando --> Seleccion: clic sobre forma
  Seleccion --> Dibujando: mover / redimensionar / color / borrar
  Dibujando --> Dibujando: Ctrl+Z / Ctrl+Y
  Dibujando --> Guardando: Guardar
  Abierto --> [*]: Cancelar (descarta)
  Guardando --> [*]: render PNG + JSON de anotaciones a Rust
```

| Herramienta | Detalle |
|---|---|
| Flecha | Color (rojo por defecto, paleta T4S), grosor |
| Número | Marcadores circulares auto-incrementales ①②③. Su significado se describe en la descripción de la evidencia |
| Texto | Caja de texto con fondo semitransparente |
| Recuadro / elipse | Contorno para señalar campos o inputs |
| Resaltado | Rectángulo amarillo translúcido |
| Desenfoque / pixelado | Para ocultar datos sensibles del cliente (pólizas, importes, PII) |
| Recortar | Sobre el original, no destructivo |

**Implementación:**
- Front con **Konva / react-konva**.
- Las anotaciones se guardan como JSON vectorial sobre el PNG **original inmutable**, así que siempre se pueden reeditar.
- Al guardar, el front genera el PNG renderizado a resolución nativa (lo que se ve es lo que sale) y Rust lo persiste.
- El Excel usa siempre el renderizado.

### 6.5 UC-16 Exportar Excel con carátula

```mermaid
sequenceDiagram
  actor T as Tester
  participant UI as React
  participant R as Rust excel/
  participant DB as SQLite + evidence/
  T->>UI: Exportar Excel
  UI->>T: Diálogo: nombre del tester (recordado), versión del documento, hojas a incluir
  T->>UI: Confirmar
  UI->>R: export_project(project_id, options)
  R->>T: diálogo nativo "Guardar como" (nombre sugerido)
  R->>DB: lee proyecto, campos visibles, casos, evidencias, versiones
  R->>R: hoja Carátula → Matriz → Evidencias → Resumen
  R-->>UI: progreso (eventos export:progress)
  R->>R: escribe .xlsx a un .tmp y luego rename atómico
  R-->>UI: ruta final
  UI->>T: toast con acciones "Abrir archivo" y "Abrir carpeta"
```

**Nombre sugerido:** `T4S_<Cliente>_<Proyecto>_Evidencias_<AAAAMMDD>_v<versión>.xlsx` (misma convención que los informes T4S).

### 6.6 UC-19 Importar Bitácora v1

```mermaid
flowchart LR
  A["Elegir matrices.json<br/>o respaldo .json v1"] --> B{"schema == 1<br/>y validación OK?"}
  B -- no --> E["Error claro:<br/>qué campo/caso falló"]
  B -- sí --> C["Por cada proyecto:<br/>nuevo PROJECT con plantilla Funcional"]
  C --> D["Mapear 13 campos v1 → PROJECT_FIELD<br/>conservar code EJ-### y next_sequence"]
  D --> F["Decodificar dataUrl base64 →<br/>evidence/*.orig.png (= render)"]
  F --> G["Todo en una transacción.<br/>Si el nombre existe: sufijo '(importado)'"]
  G --> H["Resumen: N proyectos,<br/>M casos, K capturas"]
```

### 6.7 UC-18 Compartir proyecto con un compañero

Paquete **`.qapture`** = ZIP con `project.json` (schema versionado) y `evidence/*.png`. Importar crea siempre una **copia** y nunca sobrescribe, igual que en la v1.

### 6.8 UC-21 Ciclo de vida de la ventana (segundo plano)

```mermaid
stateDiagram-v2
  [*] --> Bandeja: arranque (o inicio con Windows)
  Bandeja --> Visible: atajo Ctrl+Alt+Q / clic en bandeja
  Visible --> Bandeja: atajo Ctrl+Alt+Q / cerrar ventana (X)
  Visible --> Bandeja: Esc (si la opción está activa)
  Bandeja --> Visible: atajo de captura / pegar / nuevo caso
  Bandeja --> [*]: menú bandeja → Salir
  Visible --> [*]: menú bandeja → Salir
  note right of Bandeja
    single-instance: abrir el .exe de nuevo
    solo muestra la ventana existente
  end note
```

**Menú de bandeja:** Abrir Qapture · Capturar pantalla · Pegar evidencia · Nuevo caso · Buscar actualizaciones · Salir.

### 6.9 UC-22 Configuración

| Sección | Opciones |
|---|---|
| Perfil | Nombre del tester (se precarga en "Responsable" y en la carátula), cargo, correo opcional |
| Atajos | Grabador de combinaciones con detección de conflictos (si el SO rechaza el registro se muestra el error y se conserva el anterior). Valores por defecto: Mostrar/ocultar `Ctrl+Alt+Q` · Capturar `Ctrl+Alt+S` · Pegar evidencia `Ctrl+Alt+V` · Nuevo caso `Ctrl+Alt+N` |
| Sistema | Iniciar con Windows, cerrar = ocultar en bandeja, Esc oculta, tema claro/oscuro |
| Exportación | Carpeta por defecto, ancho máximo de imagen en Excel, incluir hoja Resumen |
| Datos | Ubicación de datos (abrir carpeta), respaldo ahora, restaurar, importar v1 |
| Acerca de | Versión, buscar actualizaciones, enlace a releases y documentación |

### 6.10 UC-23 / UC-24 Publicación y actualización

```mermaid
flowchart LR
  subgraph Dev["Desarrollo"]
    PR["Pull request"] --> CI["ci.yml<br/>lint · typecheck · vitest ·<br/>cargo fmt/clippy/test · build"]
    CI --> M["merge a main"]
    M --> TAG["git tag vX.Y.Z<br/>(o release-please)"]
  end
  subgraph Rel["release.yml (windows-latest)"]
    TAG --> B["tauri-action:<br/>build + firma updater"]
    B --> A1["Qapture_X.Y.Z_x64-setup.exe (NSIS)"]
    B --> A2["Qapture_X.Y.Z_x64_es-ES.msi"]
    B --> A3["Qapture_X.Y.Z_portable.zip"]
    B --> A4["latest.json + .sig"]
  end
  A1 & A2 & A3 & A4 --> GHR[("GitHub Release<br/>(borrador → publicado)")]
  GHR --> UPD["App instalada: al iniciar y cada 6 h<br/>consulta latest.json"]
  UPD --> ASK{"¿Instalar ahora?"}
  ASK -- sí --> INST["descarga, verifica firma,<br/>instala y reinicia"]
  ASK -- después --> BADGE["indicador en Acerca de / bandeja"]
```

- Firma del updater: `TAURI_SIGNING_PRIVATE_KEY` y su contraseña en *GitHub Secrets*. La clave pública va en `tauri.conf.json`.
- **Portable:** un ZIP con `Qapture.exe`. Guarda los datos en `.\data` junto al exe si existe un archivo `portable.flag`. No se auto-instala: avisa y enlaza a la release.
- Windows SmartScreen va a advertir porque no hay firma de código Authenticode en la v1. Queda documentado en la guía de instalación; la firma Authenticode es una mejora futura.

## 7. Diseño del Excel exportado

```mermaid
flowchart LR
  C["1 · Carátula"] --> M["2 · Matriz"]
  M <-->|"Ver evidencias / Volver"| E["3 · Evidencias"]
  M --> R["4 · Resumen"]
```

| Hoja | Contenido |
|---|---|
| **Carátula** | Banner de red de nodos (esquina superior), **logo Teams4Soft**, título "Matriz de pruebas y evidencias", bloque de datos (Cliente, Proyecto/Módulo, Tipo de prueba, Versión/Release probado, Entorno(s), Periodo inicio–fin, **Tester**, Fecha de generación), **KPI de resultados** (total de ejecuciones, por estado, % de éxito = Aprobado / ejecutados, nº de evidencias), tabla de **control de versiones** (Versión · Fecha · Elaborado · Revisado · Aprobado · Cambios) y pie "Teams4Soft · Inteligencia Híbrida · Confidencial" |
| **Matriz** | Encabezado azul `#2B629E` con texto blanco, columnas = campos visibles del proyecto en su orden y ancho, filas alternas crema `#F6F4F1`, lista desplegable de estado, formato condicional (Aprobado verde, Fallido rojo, Bloqueado amarillo), fechas `dd/mm/yyyy`, autofiltro, encabezados inmovilizados, impresión A4 horizontal ajustada al ancho |
| **Evidencias** | Por cada caso: banda con código y título, "Volver a la matriz", entorno y estado, campos clave (esperado/obtenido) y luego cada captura (descripción e imagen renderizada, ancho máximo configurable) |
| **Resumen** | Tabla por estado y por módulo/entorno (si existen esos campos) y un gráfico de barras nativo de Excel |

Todo en **Arial**. El logo y el banner van embebidos en el binario de la app (`include_bytes!`).

## 8. Manejo de errores

| Situación | Comportamiento |
|---|---|
| Error de comando Rust | `AppError { code, message_es, detail }` serializado. La UI muestra el mensaje en español y registra el detalle en el log (`tauri-plugin-log`, archivo rotado en el directorio de datos) |
| Atajo ya ocupado por otra app | Se notifica en Settings y en un toast al iniciar; se mantiene el resto de atajos |
| Disco lleno / permisos | Transacción revertida, archivos temporales limpiados, mensaje "Revisa el espacio disponible" |
| db corrupta al arrancar | Se abre en modo recuperación: ofrece restaurar desde `backups/` o importar paquete |
| Archivo Excel abierto por otro programa | "Cierra el archivo en Excel y vuelve a intentar" (se detecta el error de bloqueo de Windows) |
| Importación inválida | No escribe nada (todo o nada) y lista los primeros 10 problemas |
| Migraciones de esquema | Se versionan con `PRAGMA user_version`. Copia de seguridad automática antes de migrar |

## 9. Seguridad y privacidad

- **Sin red** salvo la consulta del updater a `github.com`. CSP estricta en `tauri.conf.json`.
- **Capabilities Tauri 2 mínimas** por ventana: el overlay solo puede emitir el rect seleccionado; la principal no tiene fs genérico, todo el I/O pasa por comandos propios con rutas validadas.
- **Repo público:** nunca se versionan datos de clientes. El `.gitignore` bloquea `*.db`, `data/`, `evidence/` y `*.xlsx`. Las semillas y los fixtures de prueba usan datos ficticios. Los datos de GMX de la v1 **no** se incluyen.
- La herramienta de desenfoque permite anonimizar capturas antes de exportar.

## 10. Estrategia de pruebas

| Nivel | Herramienta | Cubre |
|---|---|---|
| Dominio TS | Vitest | Validación por tipo de campo, `EJ-###`, resumen/KPI, reglas de plantilla (archivar vs eliminar) |
| Aplicación TS | Vitest + adaptadores `memory` | Casos de uso completos sin Tauri |
| UI | Vitest + Testing Library | Formulario dinámico por plantilla, editor de anotaciones (serialización) |
| Rust | `cargo test` | Repositorios SQLite (db en memoria), migraciones, importación legacy con el fixture v1 anonimizado, export: abrir el xlsx generado con `calamine` y verificar hojas, celdas, hipervínculos y nº de imágenes |
| E2E | WebdriverIO + `tauri-driver` (Windows) | Flujo feliz: crear proyecto → caso → pegar evidencia → exportar |
| Manual por release | Checklist en `docs/` | Abrir el Excel en Excel y WPS, atajos globales, bandeja, updater |

## 11. Fases (cada una con su propio plan de implementación)

```mermaid
gantt
  dateFormat YYYY-MM-DD
  title Roadmap Qapture v1
  section Base
  F0 Scaffold + repo + spec            :done, f0, 2026-10-15, 1d
  F1 Dominio + SQLite + CRUD UI        :f1, after f0, 6d
  section Evidencias
  F2 Pegar/arrastrar + editor anotaciones :f2, after f1, 6d
  F3 Captura integrada + overlay       :f3, after f2, 3d
  section Salida
  F4 Excel Rust con carátula + resumen :f4, after f1, 5d
  F5 Import v1 + paquete .qapture + respaldo :f5, after f4, 3d
  section Sistema y entrega
  F6 Bandeja, atajos, settings, autostart :f6, after f3, 3d
  F7 CI/CD, updater, release, docs     :f7, after f6, 3d
```

**MVP utilizable por el equipo = F1 + F2 + F4 + F6 + F7** (F3 y F5 pueden ir en v1.1 si hace falta acelerar).

## 12. Stack

| Área | Elección |
|---|---|
| Front | React 19, TypeScript, Vite, Zustand, React Router, react-hook-form + zod (formulario dinámico), react-konva, Tailwind CSS con tokens T4S |
| Core | Tauri 2, Rust 2024, rusqlite (bundled), rust_xlsxwriter, xcap, image, zip, serde, thiserror, specta / tauri-specta |
| Plugins Tauri | global-shortcut, single-instance, autostart, updater, process, dialog, clipboard-manager, notification, log, opener |
| Calidad | ESLint, Prettier, Vitest, rustfmt, clippy, WebdriverIO |
| Entrega | GitHub Actions + `tauri-apps/tauri-action`, NSIS + MSI + ZIP portable |

## 13. Preguntas abiertas (no bloquean F1)

1. **Licencia** del repo público: MIT o "todos los derechos reservados Teams4Soft". El logo es marca registrada, así que conviene aclararlo en `NOTICE`.
2. ¿El repo se mueve después a una organización GitHub `teams4soft`? Afecta la URL del updater; se puede migrar con un release puente.
3. Ícono de la app: derivarlo del isotipo (la cabeza) de Teams4Soft o diseñar uno propio de Qapture.
