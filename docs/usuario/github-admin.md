# Instalar Qapture QA en tu organización de GitHub (guía para administradores)

Esta guía es para quien administra una organización de GitHub (por ejemplo TEAMS4SOFT) y recibió una petición del equipo de QA. No necesitas conocer Qapture.

## Qué es y qué se aprueba

**Qapture** es una app de escritorio que el equipo de QA usa para documentar pruebas con capturas. Para conectarse con GitHub usa una **GitHub App** pública llamada **Qapture QA**: <https://github.com/apps/qapture-qa>.

No hay servidor intermedio: la app de escritorio habla directo con GitHub desde el equipo de cada persona, con su propia cuenta. La App no tiene webhook ni guarda contraseñas en ningún servidor.

Permisos que pedirá GitHub:

| Permiso | Nivel | Para qué |
|---|---|---|
| **Metadata** | Solo lectura | Obligatorio en toda GitHub App: ver nombre y visibilidad del repo |
| **Issues** | Lectura y escritura | Crear y comentar issues **en nombre de la persona** que usa Qapture |
| **Contents** | Lectura y escritura | Subir las capturas como archivos a un repo de evidencias |

Dos aclaraciones importantes:

- GitHub **no permite** dar *Contents* solo a un repo: se aplica a todos los repos que selecciones al instalar. Qapture **solo escribe en el repo de evidencias** que cada proyecto configure y rechaza cualquier otro destino.
- Cada acción está limitada por los permisos de la persona: nadie puede hacer con Qapture algo que no pueda hacer ya en GitHub.

## Instalar la App (si eres owner)

Sabrás si eres owner en **tu organización › People**, columna **Role**.

1. Entra a <https://github.com/apps/qapture-qa> y pulsa **Install** (o **Configure** si ya estaba instalada).
2. Elige la organización (por ejemplo **TEAMS4SOFT**).
3. Elige **Only select repositories** y marca los repos del producto (por ejemplo `GMX-frontend`, `GMX-CommonAPI`, `GMX-ClaimsAPI`) **y** el repo de evidencias `qa-evidencias`.
4. Revisa los permisos y pulsa **Install**.
5. **Importante:** en `qa-evidencias` ve a **Settings › Collaborators and teams** y da **Read** al equipo de desarrollo (o confirma que el permiso base de la organización ya da lectura). Sin esto, quien lea el issue verá las imágenes rotas.

## Si quien instala no es owner

Un *member* ve **Request** en lugar de **Install**. Al pulsarlo, GitHub envía la solicitud a los owners (a veces también por correo). Los pasos para el owner:

1. Abre **tu organización › Settings › Third-party Access › GitHub Apps › Pending requests**. GitHub cambia los nombres de sus menús de vez en cuando; si no lo encuentras, busca «GitHub Apps» en los ajustes de la organización.
2. Abre la solicitud de **Qapture QA**, revisa los repos solicitados y apruébala.
3. Haz el paso 5 de arriba (lectura de `qa-evidencias` para el equipo).

Cuando se apruebe, la persona que pidió la instalación vuelve a abrir la lista de repos en Qapture y ya los ve.

## Agregar un repo más adelante

**Tu organización › Settings › GitHub Apps › Qapture QA › Configure › Repository access**, y agrega el repo.

## Quitar el acceso

En el mismo lugar (**Configure**) puedes cambiar los repos o desinstalar la App. Cada persona también puede revocar su propia autorización en <https://github.com/settings/apps/authorizations>.

## Cuando ya está instalada

Cada tester sigue la [guía de usuario](github.md): conecta su cuenta en **Ajustes › GitHub** y vincula los repos a su proyecto.
