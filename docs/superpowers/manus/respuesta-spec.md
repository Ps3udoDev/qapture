Sí: **la arquitectura propuesta es razonable y encaja bien con Qapture**, pero haría algunos ajustes antes de aprobarla. Entiendo que cuando dices “MSP sobre STD” te refieres a **MCP sobre `stdio`**.

## Decisiones para §13

### 1. Permisos de Lectura y Escritura

Mi recomendación coincide con la tuya:

> **El interruptor general activa la integración MCP, pero todos los grupos de permisos comienzan desactivados. El tester debe habilitar manualmente cada permiso.**

No activaría Lectura y Escritura automáticamente.

La razón es que “Lectura” no necesariamente es inocua. En Qapture puede incluir casos de prueba, resultados, observaciones y capturas que podrían contener información sensible del cliente. “Escritura” permite crear casos, cambiar estados y adjuntar evidencias. Aunque el agente sea local, el cliente MCP y el modelo pueden interpretar datos de forma inesperada.

La interfaz debería mostrar cada permiso con:

- un nombre claro;
- una descripción de lo que permite;
- ejemplos de herramientas incluidas;
- una indicación del riesgo;
- el estado activo o inactivo.

Por ejemplo:

- **Lectura:** permite consultar proyectos, casos, campos y evidencias.
- **Escritura:** permite crear proyectos, crear casos, modificar casos y adjuntar evidencias.
- **Captura de pantalla:** permite capturar monitores o ventanas visibles.
- **Exportar:** permite crear archivos `.xlsx` y `.qapture` en la carpeta elegida.
- **GitHub:** permite preparar acciones relacionadas con GitHub, siempre con confirmación humana.

También recomendaría que el estado inicial fuese:

```text
MCP habilitado: sí/no
Lectura: desactivada
Escritura: desactivada
Captura: desactivada
Exportar: desactivada
GitHub: desactivada
```

La interfaz puede ofrecer un botón como **“Activar configuración recomendada”**, pero debe ser una acción explícita del usuario. No conviene que el interruptor general active permisos por detrás.

El spec ya va bien al exigir que MCP esté apagado por defecto, pero la D5 debería modificarse porque actualmente dice:

> “Al encenderlo se activan Lectura y Escritura”.

Yo la reemplazaría por:

> **D5: MCP está apagado por defecto. Al activarlo, todos los grupos de permisos permanecen apagados y el tester debe habilitar cada uno manualmente. Los permisos se muestran con nombre, descripción y herramientas afectadas.**

Esto está alineado con la recomendación oficial de MCP de mantener al usuario dentro del circuito de decisión y permitirle rechazar invocaciones sensibles ([MCP Tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools)).

### 2. Creación de proyectos nuevos

Aquí decidiría:

> **El agente sí puede crear proyectos nuevos, siempre que el permiso “Escritura” esté habilitado.**

Tiene sentido para el escenario de migración descrito en el spec. Si el agente debe documentar una migración completa, obligarlo a trabajar únicamente sobre proyectos creados previamente introduce una fricción artificial. El usuario tendría que abandonar el flujo para crear manualmente el proyecto antes de iniciar el trabajo del agente.

También es coherente con que el spec ya incluya `create_project` dentro de las herramientas de Escritura.

Aun así, pondría algunos límites:

1. El agente solo puede crear proyectos usando una plantilla existente mediante `templateId`.
2. No puede crear ni modificar plantillas desde MCP.
3. No puede borrar proyectos.
4. El nombre y los metadatos deben validarse igual que en la interfaz.
5. La actividad debe registrar quién creó el proyecto y desde qué cliente MCP.
6. La respuesta de `create_project` debería devolver claramente el identificador, nombre, plantilla y fecha de creación.
7. El prompt `documentar_migracion` debería pedir confirmación del proyecto antes de empezar, pero no necesariamente confirmación manual para cada caso.

Conviene distinguir entre **permiso de Escritura** y **confirmación interactiva por operación**. Para un flujo de diez o más casos, pedir confirmación manual para cada `create_case` haría que la automatización perdiera gran parte de su valor. Es suficiente que el usuario active Escritura conscientemente y que exista un registro auditable.

Para operaciones más delicadas —por ejemplo, exportar datos, capturar pantalla o preparar una publicación en GitHub— sí mantendría confirmaciones específicas. El spec ya lo hace correctamente con GitHub.

### 3. Linux para cerrar F9a

Aquí separaría dos cosas:

- **cerrar F9a como funcionalidad MCP**;
- **afirmar que el soporte Linux está validado**.

Mi recomendación sería:

> **F9a puede cerrarse con Windows como plataforma principal, pero no debe considerarse validado el soporte Linux hasta ejecutar una prueba real en Linux.**

El spec tiene una pequeña contradicción:

- en §11, la puerta de salida de F9a habla de Claude Code y Codex en Windows;
- en §9, el checklist manual ya incluye Linux AppImage.

Conviene resolverla explícitamente. Yo dejaría F9a así:

> **Puerta de salida obligatoria:** escenario completo con Claude Code y Codex en Windows.  
> **Validación adicional:** prueba de humo del AppImage en al menos un entorno Linux compatible antes de publicar soporte Linux como estable.

Exportar y construir el `.AppImage` **no es suficiente por sí solo**. Eso confirma que el artefacto se generó, pero no confirma que:

- la aplicación arranque;
- `qapture-mcp` sea ejecutable;
- el puente pueda iniciar Qapture;
- el socket Unix se cree correctamente;
- los permisos de `mcp.key` sean correctos;
- el agente pueda completar `initialize`, `tools/list` y `tools/call`;
- SQLite y las evidencias funcionen;
- la captura de pantalla funcione bajo el entorno gráfico;
- el paquete exportado se genere correctamente.

La documentación de Tauri confirma que AppImage facilita la distribución, pero también advierte que la compatibilidad depende de la versión base del sistema y de bibliotecas como `glibc`. Tauri recomienda construir sobre una base antigua compatible, como Ubuntu 22.04 o Debian 12, y probar el resultado en los sistemas objetivo ([Tauri AppImage](https://v2.tauri.app/distribute/appimage/)). La documentación de AppImage también señala que las diferencias entre distribuciones hacen necesario probar los artefactos en los entornos que se pretenden soportar ([AppImage testing](https://docs.appimage.org/introduction/advantages.html)).

Por eso, en tu caso sí sería válido:

1. generar el `.AppImage`;
2. entregárselo a un tester con Linux o ejecutarlo en una máquina virtual;
3. pedirle que realice el escenario MCP completo;
4. registrar distribución, versión, escritorio y resultado;
5. corregir cualquier problema antes de declarar Linux soportado.

Una máquina virtual puede ser suficiente para una primera validación. También puede usarse CI para comprobar que el binario arranca, aunque las pruebas de captura de pantalla y ventanas necesitan un entorno gráfico real o cuidadosamente simulado. Para la primera prueba recomendaría Ubuntu 22.04 o 24.04 con escritorio, porque el stack de Tauri ya contempla ese entorno.

## Sobre MCP dentro de la aplicación

La decisión D2 me parece buena. El servidor MCP no debería abrir `qapture.db` por separado. Qapture ya concentra en el proceso Rust el acceso a SQLite, las migraciones, el modo recuperación, las evidencias y los eventos hacia la interfaz. Añadir otro proceso escritor aumenta el riesgo de conflictos, estados desactualizados y errores de sincronización.

La separación propuesta es adecuada:

```text
Cliente MCP
   ↓ stdio
qapture-mcp
   ↓ named pipe / Unix socket
Qapture
   ↓
repositorios, SQLite, evidencias y UI
```

`qapture-mcp` funciona como un adaptador de transporte y no como una segunda aplicación con lógica de negocio. Esto aprovecha correctamente el modelo de `stdio`: el cliente inicia un proceso hijo y se comunica mediante `stdin` y `stdout`; el servidor no debe escribir logs en `stdout`, porque esos bytes deben ser mensajes MCP válidos ([MCP Transports](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports)).

La solución también es coherente con la arquitectura actual de Qapture: Tauri 2, React, Rust, SQLite y almacenamiento de evidencias en archivos ([repositorio de Qapture](https://github.com/Ps3udoDev/qapture)). La documentación de Tauri confirma que los binarios auxiliares pueden empaquetarse como *sidecars* y que deben distribuirse con el sufijo correspondiente al *target triple* ([Tauri Sidecar](https://v2.tauri.app/develop/sidecar/)).

Solo añadiría estas precauciones al spec:

- definir qué ocurre si `qapture-mcp` y Qapture tienen versiones incompatibles;
- incluir una versión de protocolo interno en el primer mensaje del canal local;
- cerrar correctamente conexiones cuando Qapture se actualiza o reinicia;
- impedir que dos instalaciones portables compartan accidentalmente el mismo socket;
- registrar el cliente MCP, pero no incluir secretos, rutas sensibles ni contenido de imágenes en `agent_activity`;
- separar los errores de “MCP desactivado”, “permiso denegado” y “app no disponible”;
- decidir si las herramientas sin permiso aparecen en `tools/list` o si se ocultan.

Yo prefiero que las herramientas no autorizadas **no se ofrezcan al cliente** o aparezcan claramente marcadas como deshabilitadas. Devolverlas siempre y fallar después puede inducir al agente a reintentar continuamente.

## Conclusión

Aprobaría el diseño con estas tres decisiones:

1. **Permisos:** todo apagado inicialmente; el tester activa Lectura, Escritura y los demás grupos manualmente, con etiqueta y descripción.
2. **Proyectos:** el agente puede crear proyectos nuevos mediante `create_project`, dentro del permiso de Escritura y usando plantillas existentes.
3. **Linux:** Windows es suficiente para cerrar la aceptación obligatoria de F9a, pero Linux debe tener una prueba real del AppImage antes de afirmar que el soporte Linux está validado.

La frase que podrías responder al equipo sería:

> Para §13, propongo que al activar MCP no se habilite ningún permiso automáticamente. Lectura, Escritura, Captura, Exportar y GitHub deben quedar desactivados hasta que el tester los marque de forma explícita, mostrando el nombre y la descripción de lo que permite cada uno. El agente sí debe poder crear proyectos nuevos usando plantillas existentes, siempre dentro del permiso de Escritura y sin capacidad de borrado. Para F9a, la aceptación obligatoria puede hacerse en Windows; Linux no debería bloquear el cierre funcional si se considera una plataforma secundaria, pero el AppImage debe probarse en una máquina Linux real o virtual antes de declarar soporte Linux. Construirlo no basta: hay que validar arranque, puente MCP, socket, permisos, captura, persistencia y exportación.