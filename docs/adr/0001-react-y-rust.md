# ADR 0001: React para lo visual, Rust para el núcleo

**Fecha:** 2026-10-15 · **Estado:** vigente · Decisión D6 del [spec](../superpowers/specs/2026-10-15-qapture-design.md).

## Contexto

Qapture es una app de escritorio que necesita una interfaz rica (formularios dinámicos y un editor de anotaciones sobre canvas) y, a la vez, trabajo del sistema: captura de pantalla, archivos, bandeja, atajos globales, Excel y actualizaciones. Tauri 2 permite mezclar un WebView con un núcleo en Rust.

## Decisión

React 19 + TypeScript para todo lo visual, incluido el editor de anotaciones (Konva). Rust para captura, almacenamiento, archivos, Excel, bandeja, atajos y updater. El WebView solo llega al sistema mediante comandos propios, con permisos mínimos por ventana.

## Consecuencias

- El WebView no tiene acceso genérico al disco ni a la red; la superficie de ataque es pequeña.
- Hay dominio en dos lenguajes: Rust revalida todo, y el dominio TS existe para dar respuesta inmediata en la interfaz.
- Hace falta cuidar que ambos lados no se desalineen (ver [ADR 0004](0004-tests-de-contrato.md)).
- Se necesita el toolchain de Rust además de Node para desarrollar.
