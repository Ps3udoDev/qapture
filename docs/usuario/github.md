# Conectar Qapture con GitHub

Esta guía explica cómo conectar tu cuenta de GitHub y vincular repositorios a un proyecto. Es la primera parte de la integración: **reportar un caso como issue, importar issues y mantener los estados al día llegan en versiones posteriores**. Hoy puedes conectar, desconectar y dejar los proyectos listos para ese momento.

Qapture habla con GitHub solo cuando tú lo pides. Sin conexión, el resto de la app funciona igual.

## Antes de empezar

Necesitas una cuenta de GitHub y que la App **Qapture QA** (`qapture-qa`) esté instalada en la cuenta u organización cuyos repos quieres usar:

- **Tus repos personales:** instálala tú mismo desde **Ajustes › GitHub › Instalar o configurar Qapture en una cuenta u organización**.
- **Repos de una organización:** la instalación la hace (o aprueba) un owner de la organización. Pásale la [guía para el administrador](github-admin.md).

## Conectar tu cuenta

1. Abre **Ajustes › GitHub** y pulsa **Conectar con GitHub**.
2. Qapture muestra un **código** grande. Pulsa **Copiar código** y luego **Abrir GitHub** (se abre `github.com/login/device`).
3. Pega el código en GitHub y autoriza a Qapture QA.
4. Vuelve a Qapture: en unos segundos aparece tu avatar y **@tu-usuario**.

El código vence en pocos minutos; la pantalla muestra la cuenta regresiva. Si vence, Qapture avisa «El código venció. Vuelve a pulsar «Conectar con GitHub».»: pulsa **Conectar con GitHub** otra vez. **Cancelar** detiene el proceso. Si se corta la red un momento mientras esperas, Qapture sigue intentando hasta que el código vence.

Tu sesión de GitHub se guarda en el **llavero del sistema** (Administrador de credenciales de Windows, o Secret Service en Linux). Nunca queda en un archivo, ni en los respaldos ni en los paquetes `.qapture`.

## Ver, instalar y revocar

Con la cuenta conectada, en **Ajustes › GitHub** verás:

- **Instalar o configurar Qapture en una cuenta u organización:** abre la página de GitHub para elegir en qué repos funciona Qapture.
- **Revocar acceso en GitHub:** abre la página donde GitHub lista las apps autorizadas y puedes quitar a Qapture QA.
- **Desconectar:** pide confirmación, borra la sesión del llavero y quita tu cuenta de Qapture. Los repos vinculados a tus proyectos **se conservan**. Si el llavero no responde, Qapture quita igual tu cuenta y te avisa del error del llavero.

## Reconectar

Si revocaste el acceso en GitHub, o la sesión dejó de ser válida, Qapture lo detecta la próxima vez que consulta GitHub y muestra **Reconectar**. Pulsa ese botón y repite los pasos de «Conectar tu cuenta». No pierdes nada de tus proyectos.

## Vincular repos a un proyecto

Abre el proyecto y entra a **Configurar proyecto › GitHub**. Los controles se activan cuando termina de cargar la configuración guardada; si no carga, la pestaña lo avisa y no deja guardar (así no se pierde lo guardado). Cierra y vuelve a abrir la pestaña para intentarlo de nuevo.

- **Repositorios vinculados:** pulsa **Agregar repositorio** y elige de la lista (solo aparecen los repos donde Qapture QA está instalada y tú tienes acceso). Uno debe quedar marcado como **Predeterminado**. Cada repo lleva sus **etiquetas** por defecto (por ejemplo `bug, qapture`) y se quita con **Quitar**.
- **¿No ves tu repo?** Copia el enlace de instalación para abrirlo tú o mandárselo al administrador, y abre la [guía para el admin](github-admin.md) en el navegador. Cuando esté instalada, vuelve a abrir la lista.
- **Ruteo por columna (opcional):** elige una columna de tipo **Lista** de tu proyecto (por ejemplo «Módulo») y asigna a cada opción el repo que le corresponde. Cuando reportar issues esté disponible, sugerirá el repo según esa columna. Si más adelante archivas la columna o cambias su tipo, la pestaña te avisa y el proyecto sigue abriendo normal.
- **Repo de evidencias (opcional):** un repo dedicado donde se guardarán las capturas de los reportes. Si eliges **Ninguno: pegar capturas a mano**, tú pegarás las imágenes en el issue.

Guarda los cambios. Qapture revisa que haya un repo predeterminado, que no haya repos repetidos y que el ruteo apunte a repos vinculados.

## Qué todavía no hace

- Crear issues desde un caso, con las capturas anotadas.
- Importar issues de GitHub como casos.
- Sincronizar el estado de casos e issues.
- Llevar los vínculos de GitHub dentro de un paquete `.qapture`: por ahora el paquete no incluye datos de GitHub.

Si algo no funciona, revisa las [preguntas frecuentes](faq.md#github).
