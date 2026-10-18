# Arnés de captura

Genera los insumos del video a partir de la app real.

- `demo.html` + `demo-main.tsx`: montan la UI de Qapture sobre los repositorios en memoria con un
  proyecto de ejemplo (`?scene=workspace|editor-new|editor-edit`). Con `pnpm dev` en la raíz del repo
  se abren en `http://localhost:1420/videos/qapture-promo/capture/demo.html`.
- `sut.html`: pantalla ficticia del "sistema bajo prueba" que se usa como evidencia (`sut.png`).
- `synth_audio.py`: sintetiza la música y los efectos (`python synth_audio.py ../assets/audio`,
  requiere `numpy`, `scipy` y `soundfile`; luego `music.wav` se convierte a `music.mp3` con ffmpeg).

Las tomas de `assets/shots/` se sacaron con Chrome headless a 1440×900 y escala 2×. La voz se generó
con `npx hyperframes tts` (Kokoro, voz `ef_dora`, velocidad 1.05).
