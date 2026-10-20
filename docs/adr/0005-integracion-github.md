# ADR 0005: Integración con GitHub sin backend (GitHub App + Device Flow)

**Fecha:** 2026-10-20 · **Estado:** vigente · Decisiones D1–D13 del [spec](../superpowers/specs/2026-10-19-integracion-github-design.md).

## Contexto

Qapture es una app de escritorio local, sin servidor propio. Queremos que cada tester use su cuenta de GitHub para reportar fallos como issues con capturas, también en organizaciones privadas, sin pedir a QA que cree tokens a mano y sin montar un backend que guarde secretos.

## Decisión

1. **GitHub App con Device Flow, sin backend (D1).** La App pública `qapture-qa` (bajo `Ps3udoDev`, D2) solo necesita su `client_id`, que es público y va en el código. No hay `client_secret` ni webhook. La llave privada que GitHub obliga a generar para instalar la App se guarda fuera del repo y Qapture nunca la usa.
2. **Permisos mínimos que alcanzan (D3):** Metadata (lectura), Issues y Contents (lectura y escritura). Contents lo exige subir capturas; GitHub no permite limitarlo a un repo, así que Qapture se obliga a escribir solo en el repo de evidencias del proyecto.
3. **Tokens sin expiración (D4).** Evita necesitar un `client_secret` para refrescar. Ante un 401 la cuenta pasa a «Reconectar» y se repite el Device Flow.
4. **Token en el llavero del sistema (D11).** Credential Manager en Windows y Secret Service en Linux, con el crate `keyring`. Nunca en archivo, respaldo ni paquete. No hay plan B si falta el llavero: la conexión falla con un mensaje que dice qué instalar.
5. **Llamadas solo desde Rust (D12).** `reqwest` con rustls; el CSP del WebView no cambia (el avatar viaja como `data:`).
6. **Repo de evidencias dedicado (D6, D7) con modo asistido (D8).** Las capturas se subirán a un repo propio, con una carpeta por proyecto, caso y fecha y un commit por reporte, y los issues las enlazarán por el SHA del commit. Si no hay repo de evidencias, el tester pega las imágenes a mano guiado por la app.
7. **Varios repos por proyecto (D5, D10)** con uno predeterminado y ruteo opcional por una columna de tipo Lista; como máximo un issue por caso y repo.
8. Solo `github.com` (D13).

## Por qué no se imita el Ctrl+V de la web

Al pegar una imagen en un issue en github.com, el navegador la sube con un endpoint interno que no es parte de la API pública: depende de la sesión web y no de tokens de App, y puede cambiar sin aviso. Imitarlo exigiría manejar la sesión del navegador, algo frágil e inseguro. La API pública sí permite guardar archivos en un repo, por eso se eligió el repo de evidencias, y el modo asistido cubre el resto.

## Consecuencias

- No hay servidor que mantener ni secretos que custodiar fuera del equipo del tester.
- Instalar la App en una organización requiere a un owner (flujo «Request»); por eso existe la [guía para administradores](../usuario/github-admin.md).
- Contents en lectura y escritura sobre los repos instalados es un permiso amplio; se mitiga con la barrera de destino en Qapture y con que GitHub limita cada acción a los permisos del usuario.
- La transferencia de la App a la organización queda para después (D2) y no cambia el `client_id` de uso.
- Reportar (F8b), importar (F8c) y sincronizar (F8d) se construyen sobre esta base en versiones posteriores.
