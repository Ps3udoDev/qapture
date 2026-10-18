# ADR 0003: SQLite para los datos y archivos PNG para las imágenes

**Fecha:** 2026-10-15 · **Estado:** vigente · Decisión D8 del [spec](../superpowers/specs/2026-10-15-qapture-design.md).

## Contexto

La Bitácora v1 guardaba todo en un JSON con las capturas en base64, que crece sin control y se corrompe con facilidad. Qapture necesita datos estructurados con transacciones, migraciones y respaldos, y muchas imágenes pesadas.

## Decisión

Los datos estructurados van en **SQLite** (`rusqlite`, empaquetado con la app), versionado con `PRAGMA user_version`. Las imágenes son **archivos PNG** en el directorio de datos (`evidence\`), referenciados desde la base: el original y el renderizado con anotaciones.

## Consecuencias

- Operaciones atómicas: o se guarda el caso con su evidencia, o nada.
- Migraciones con copia de seguridad automática y respaldos diarios de la base.
- La base y la carpeta de imágenes deben moverse juntas (por eso existen los paquetes `.qapture` y los respaldos).
- La base no guarda imágenes, así que se mantiene pequeña y rápida.
- La ubicación depende del modo: instalada en `%APPDATA%\com.teams4soft.qapture`, portable en `data\` junto al exe.
