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

Todo funciona igual. Solo la búsqueda de actualizaciones necesita conexión.

## Windows me muestra «Windows protegió su PC»

Es el aviso de SmartScreen. Mira los pasos en [Instalación](instalacion.md#el-aviso-de-smartscreen).
