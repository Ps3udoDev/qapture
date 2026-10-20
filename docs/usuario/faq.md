# Preguntas frecuentes

## Un atajo no funciona o dice «No registrado»

Otra aplicación ya está usando esa combinación, o el teclado la interpreta como otra cosa (ver la nota de AltGr en [Atajos](atajos.md#nota-para-teclados-en-español-altgr)). Ve a **Ajustes › Atajos**, elige otra combinación y guarda. El resto de atajos sigue funcionando.

## Cerré la ventana y Qapture «desapareció»

No se cerró: **vive en la bandeja del sistema** (los íconos junto al reloj; puede estar bajo la flecha ^ de «iconos ocultos»). Para volver a abrirla:

- haz clic en el ícono de Qapture, o
- presiona `Ctrl+Alt+Q`, o
- abre el programa otra vez (solo muestra la ventana que ya existe).

Para cerrarla de verdad: clic derecho en el ícono › **Salir**. Si prefieres que la X cierre la app, desactiva **Ajustes › Sistema › Cerrar oculta en la bandeja**. También puedes activar **Iniciar con Windows**.

Si la ventana se oculta al presionar `Esc`, es la opción **Ajustes › Sistema › Esc oculta la ventana**: desactívala si no la quieres. La app sigue en la bandeja igual que al cerrar con la X.

## Tengo la versión portable y la instalada: ¿pasa algo?

Qapture permite **una sola ejecución a la vez**, y eso incluye a las dos versiones: si una está abierta y abres la otra, verás la ventana de la primera. Sus datos son distintos (la instalada usa `%APPDATA%\com.teams4soft.qapture`; la portable, su carpeta `data`), así que no se mezclan. Cierra una con **Salir** en la bandeja antes de abrir la otra.

## Algo falló y Qapture no abre mis datos

Aparece la pantalla «Qapture no pudo abrir tus datos». Elige **Volver a un respaldo** y escoge el más reciente. Con la app funcionando también puedes ir a **Datos › Respaldos…** y **Restaurar**. Antes de restaurar, Qapture guarda una copia de lo que había. Las capturas que no pertenezcan a ningún proyecto restaurado se apartan en la carpeta `evidence-sin-uso`, dentro de la carpeta de datos, y no se borran.

## ¿Cómo paso mis datos a otro equipo?

Elige la opción que te sirva:

- **Por proyecto (recomendado):** en el equipo viejo, **Datos › Exportar paquete .qapture…** por cada proyecto. En el equipo nuevo, **Datos › Importar paquete .qapture…**.
- **Todo de una vez:** con Qapture cerrada en ambos equipos, copia la carpeta de datos (`%APPDATA%\com.teams4soft.qapture`, o `data` en la portable) a la misma ubicación del equipo nuevo. Puedes abrirla desde **Ajustes › Datos › Abrir carpeta de datos**.

## ¿Qué pasa si no tengo internet?

Todo funciona igual. Solo la búsqueda de actualizaciones y las acciones de GitHub necesitan conexión.

## Windows me muestra «Windows protegió su PC»

Es el aviso de SmartScreen. Mira los pasos en [Instalación](instalacion.md#el-aviso-de-smartscreen).

## GitHub

### No veo mi repo en la lista

Qapture solo muestra los repos donde la App **Qapture QA** está instalada y tú tienes acceso. En **Configurar proyecto › GitHub** pulsa **¿No ves tu repo?** para copiar el enlace de instalación. En un repo personal la instalas tú; en una organización la instala o aprueba un owner (pásale la [guía para administradores](github-admin.md)). Si eres *member* y GitHub muestra **Request**, espera la aprobación y vuelve a abrir la lista.

### Error de llavero en Linux

Qapture guarda tu sesión de GitHub en el llavero del sistema (Secret Service). Si ves «No se encontró un llavero del sistema», instala y desbloquea **gnome-keyring** o **KWallet** (en escritorios mínimos no vienen) y vuelve a pulsar **Conectar con GitHub**. Qapture no guarda la sesión en un archivo como alternativa. En Windows el mensaje equivalente habla del Administrador de credenciales.

### Cómo revoco el acceso

Tienes dos caminos: **Ajustes › GitHub › Desconectar** (borra la sesión de este equipo) y, para quitar la autorización del lado de GitHub, **Revocar acceso en GitHub** (<https://github.com/settings/apps/authorizations>). Si revocas en GitHub, Qapture te mostrará **Reconectar** la próxima vez. Más detalles en [Conectar con GitHub](github.md).

### Las imágenes del issue salen rotas

Las capturas viven en el repo de evidencias del proyecto, y solo las ve quien tenga acceso de **lectura** a ese repo. Pide al administrador que dé **Read** a tu usuario o a tu equipo en el repo de evidencias (ver la [guía para administradores](github-admin.md)).

### Repo de evidencias vacío

Si ves «El repo de evidencias o/r está vacío: crea un primer archivo (por ejemplo, un README) y vuelve a intentar.», crea un archivo cualquiera en ese repo desde GitHub y reporta de nuevo. No se creó ningún issue a medias.

### Me pide pegar las capturas a mano

Es el modo asistido: no hay repo de evidencias, tu cuenta no puede escribir en él o la App no está instalada ahí. Sigue el panel **Termina de adjuntar las capturas** (más detalle en [Conectar con GitHub](github.md#modo-asistido)). El aviso de capturas sin pegar se pierde al cerrar la app.
