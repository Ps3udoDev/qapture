# Uso: de la captura al Excel

Los ejemplos usan datos inventados (proyecto «Tienda Demo»). Con datos reales de un cliente, mantenlos en tu equipo y nunca los subas a un repositorio.

## 1. Crear un proyecto (o importar la Bitácora v1)

Un proyecto es una matriz de pruebas, por ejemplo la de un release.

- **Nuevo:** en la barra superior, **Nueva matriz** (o «Crea tu primera matriz» si es la primera vez). Escribe el nombre y elige una **Plantilla de columnas** (Funcional, Regresión, API o UAT). Después puedes ajustar columnas, datos del proyecto y versiones del documento en **Configurar**.
- **Viene de la Bitácora v1:** **Datos › Importar Bitácora v1…** y elige tu `matrices.json` o el respaldo `.json` de la v1. Se crea un proyecto por cada matriz que traía, con sus casos y capturas. Si algo no es válido, Qapture no importa nada y te lista los primeros problemas.

## 2. Registrar casos

Con el proyecto abierto, en **Casos de prueba** pulsa **Nuevo caso** y llena el formulario. Cada caso recibe un código automático con el formato que usa la app, por ejemplo `EJ-001`, `EJ-002`…, y empieza en **No ejecutado**.

El estado cambia durante la prueba: No ejecutado, En ejecución, Por documentar, Aprobado, Fallido o Bloqueado. Usa el buscador («Buscar código, caso o dato…») y el filtro de estado para encontrar casos.

## 3. Capturar evidencias

Abre el caso y, en **Evidencias**, usa uno de estos caminos:

- **Capturar** (botón), o el atajo **`Ctrl+Alt+S`** desde cualquier programa aunque Qapture esté oculta: la pantalla se congela y arrastras para elegir una región. `Enter` captura el monitor completo y `Esc` cancela.
- **Recortar con Windows y pegar:** presiona `Win+Shift+S`, recorta como siempre y luego presiona **`Ctrl+Alt+V`**. Qapture abre la imagen en el editor. Si el portapapeles no tiene una imagen, aparece un aviso y no se abre nada.
- **Elegir imagen** para tomar un archivo, o arrástralo a la zona de evidencias.

Con los atajos (`Ctrl+Alt+S` o `Ctrl+Alt+V`), Qapture muestra su ventana y abre la captura en el editor con el caso seleccionado ya elegido; si no hay ninguno seleccionado, lo eliges en el campo **Caso** del editor antes de guardar. Si el proyecto todavía no tiene casos, aparece el aviso «Crea un caso antes de capturar evidencias.» y la captura se descarta. Si el editor ya está abierto, la captura nueva espera y se abre al guardar o descartar la actual.

## 4. Anotar

Cada captura se abre en el editor. Las herramientas están en su barra (la letra entre paréntesis es su atajo): Seleccionar (V), Flecha (A), Número (N), Texto (T), Recuadro (R), Elipse (E), Resaltado (H), Desenfoque (B) y Recortar (C). Hay Deshacer (`Ctrl+Z`) y Rehacer (`Ctrl+Y`).

Usa **Desenfoque** para ocultar datos personales o sensibles antes de exportar. Al guardar, la evidencia queda en el caso.

## 5. Exportar el Excel

1. En la barra superior, **Exportar**.
2. Elige **Matriz con evidencias** (carátula, matriz, capturas y resumen) o **Plantilla vacía** (carátula y columnas del proyecto, sin casos).
3. Revisa el **Tester** (se toma de **Ajustes › Perfil**) y elige o crea la **Versión del documento**.
4. Pulsa **Exportar Excel** (o **Exportar plantilla**).

Si el archivo está abierto en Excel, ciérralo y vuelve a intentar. En **Ajustes › Exportación** puedes fijar la carpeta por defecto, el ancho máximo de las imágenes y si se incluye la hoja Resumen.

## Paquetes .qapture y respaldos

Todo está en el menú **Datos** de la barra superior.

- **Compartir un proyecto:** **Exportar paquete .qapture…** crea un archivo con el proyecto y sus capturas. Tu compañero lo abre con **Importar paquete .qapture…**. Siempre se crea una **copia**: nunca sobrescribe lo que ya tiene.
- **Respaldos…** lista las copias de seguridad. Qapture crea una por día (conserva las últimas siete) y otra automática antes de actualizar la estructura de datos o de restaurar. Puedes crear una manual cuando quieras y **Restaurar** cualquiera. Antes de restaurar se guarda una copia de lo actual.

Más detalle de recuperación y cambio de equipo en las [preguntas frecuentes](faq.md).
