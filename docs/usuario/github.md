# Conectar Qapture con GitHub

Esta guía explica cómo conectar tu cuenta de GitHub y vincular repositorios a un proyecto. Con ella puedes conectar tu cuenta, vincular repos a un proyecto y **reportar un caso como issue** (o comentar un re-test) con las capturas incluidas. **Importar issues y mantener los estados al día llegan en versiones posteriores**.

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
- **Ruteo por columna (opcional):** elige una columna de tipo **Lista** de tu proyecto (por ejemplo «Módulo») y asigna a cada opción el repo que le corresponde. Al reportar un caso, Qapture sugiere el repo según esa columna. Si más adelante archivas la columna o cambias su tipo, la pestaña te avisa y el proyecto sigue abriendo normal.
- **Repo de evidencias (opcional):** un repo dedicado donde Qapture sube las capturas de los reportes (ver «Dónde quedan las capturas»). Si eliges **Ninguno: pegar capturas a mano**, Qapture usa el modo asistido y tú pegas las imágenes en el issue.

Guarda los cambios. Qapture revisa que haya un repo predeterminado, que no haya repos repetidos y que el ruteo apunte a repos vinculados.

## Reportar un caso

Abre el detalle del caso y pulsa **Reportar en GitHub**. Si no has conectado tu cuenta, Qapture te lo dice («Conecta tu cuenta de GitHub en Ajustes › GitHub.», con el botón **Abrir Ajustes**); si el proyecto no tiene repos, muestra «Este proyecto no tiene repositorios vinculados.» con el botón **Vincular repositorio**.

El diálogo trae:

- **Repositorio:** ya elegido según la columna de ruteo del caso o, si no aplica, el repo predeterminado.
- **Título:** `[código] título del caso`.
- **Descripción:** texto generado con proyecto, estado, quién reporta y los campos del caso. Tiene las pestañas **Editar** y **Vista previa** (la vista previa se muestra como texto plano). **Regenerar descripción** vuelve a armarla desde el caso.
- **Etiquetas**, **Asignados** y **Milestone** (puede quedar en «Sin milestone»), tomados del repo elegido. Hasta 10 etiquetas y 10 asignados.
- **Capturas:** todas vienen marcadas, con miniaturas; desmarca las que no quieras. Cada captura marcada muestra su número («Captura 1», «Captura 2»…), que es el de su marca `{{img:n}}` en la descripción. Se envían **ya renderizadas**, con el difuminado y las anotaciones. Si una pesa más de 10 MB, Qapture avisa «La captura n pesa más de 10 MB; GitHub podría rechazarla.» (con ese mismo número), pero no te lo impide.

  Al marcar o desmarcar capturas, si no tocaste la descripción, Qapture la vuelve a armar sola para que las leyendas coincidan. Si ya la editaste, no la cambia: muestra «Cambiaste las capturas: regenera la descripción para que coincidan.» junto a **Regenerar descripción** (al regenerarla se pierden tus cambios al texto).

Debajo, una línea indica el destino: «Se creará en o/r como @tu-usuario.». Pulsa **Crear issue**. Qapture muestra «Subiendo capturas…» y luego «Creando issue…», y al terminar avisa «Issue #n creado en o/r.».

Las marcas `{{img:1}}`, `{{img:2}}`… de la descripción son los lugares donde se insertan las capturas. Si las borras, las reordenas o escribes un número que no existe, Qapture lo corrige: cada captura elegida aparece una sola vez y no queda ninguna marca suelta.

### Un issue por caso y repo

Cada caso tiene como máximo un issue por repo, pero puede tener issues en varios repos (por ejemplo, uno en el frontend y otro en la API). Después de reportar, el caso muestra un chip `repo#n` por cada issue; al pulsarlo se abre en GitHub. En la lista de casos, un ícono marca los que ya tienen issues.

### Re-test: comentar en el issue existente

Si el caso ya tiene un issue en el repo elegido, el diálogo pasa a **Comentar en o/r#n**. Trae una plantilla de re-test («Re-test el AAAA-MM-DD por tester: estado …») para que agregues tus notas y las capturas nuevas. El botón es **Comentar** y avisa «Comentario agregado en o/r#n.».

## Dónde quedan las capturas

Qapture sube las capturas elegidas en **un solo commit** al **repo de evidencias** del proyecto (rama predeterminada), con esta ruta:

`<proyecto>/<caso>/<AAAA-MM-DD_HHmmss>_<tester>/<n>.png`

y las enlaza en el issue con una dirección de la forma `github.com/<o>/<r>/blob/<commit>/<ruta>?raw=true`, que es la que funciona con repos privados.

**Importante:** esas imágenes solo las ve quien tenga acceso de **lectura al repo de evidencias**. Quien no lo tenga verá la imagen rota en el issue. Pídele al administrador que dé lectura a todo el equipo (ver la [guía del admin](github-admin.md)).

Si el repo de evidencias está vacío, Qapture avisa «El repo de evidencias o/r está vacío: crea un primer archivo (por ejemplo, un README) y vuelve a intentar.» y no crea nada.

## Modo asistido

Si Qapture no puede subir las capturas, el issue se crea igual. Pasa cuando:

- el proyecto no tiene repo de evidencias («No hay repo de evidencias configurado.»),
- tu cuenta no puede escribir en él («Tu cuenta no puede escribir en o/r.»), o
- la App no está instalada en ese repo («Qapture no está instalada en o/r.»).

En el issue, cada captura queda como «📎 Evidencia n: pendiente de pegar», y Qapture abre el panel **Termina de adjuntar las capturas**:

1. Pulsa **Copiar** en una captura («Captura n copiada. Pégala con Ctrl+V en GitHub.»).
2. Pégala con `Ctrl+V` en un comentario del issue (o editando la descripción).
3. Marca la casilla **Pegada**.

**Abrir issue** abre el issue en GitHub y **Cerrar** cierra el panel. Mientras queden capturas sin pegar, el caso muestra el aviso «k capturas sin pegar». Ese recordatorio **solo dura mientras la app siga abierta**; si la cierras, se pierde (las capturas siguen en el caso y puedes copiarlas cuando quieras).

## Excel y paquetes

- **Excel:** si algún caso del proyecto tiene issues, el archivo agrega la columna **Issue** después de **Evidencias**. Con un issue, la celda es un hipervínculo `repo#n` (si la dirección guardada no es de `https://github.com`, queda solo el texto, sin enlace); con varios, muestra `repo#n, repo#m` y una nota de celda con una dirección por línea.
- **Paquetes `.qapture`:** el paquete lleva los datos de GitHub (repos, ruteo y vínculos con sus issues) en un archivo opcional `github.json`. Al importarlo en otra instalación se conservan los vínculos. Una versión 1.0 de Qapture abre el paquete igual, sin los datos de GitHub. Si el archivo está dañado, o algún vínculo no apunta a su issue en `https://github.com/dueño/repo/issues/n`, el proyecto se importa sin ellos y Qapture avisa «No se pudieron leer los datos de GitHub del paquete; el proyecto se importó sin ellos.».
- **El repo de evidencias no se importa:** lo confirma cada equipo. Si el paquete traía uno, Qapture avisa «El paquete traía un repo de evidencias; confírmalo en Configurar proyecto › GitHub.».

## Mensajes frecuentes

- «El caso ya tiene un issue en o/r (#n).»: ya reportaste ese caso en ese repo; usa el chip o comenta el re-test.
- «Ya se está creando un issue de este caso en o/r.» / «Ya se está comentando este caso en o/r.»: hay otro envío en curso; espera a que termine.
- «El caso no tiene un issue en o/r.»: quisiste comentar en un repo donde el caso aún no tiene issue; reporta primero y comenta después.

## Qué todavía no hace

- Importar issues de GitHub como casos.
- Sincronizar el estado de casos e issues.

Si algo no funciona, revisa las [preguntas frecuentes](faq.md#github).
