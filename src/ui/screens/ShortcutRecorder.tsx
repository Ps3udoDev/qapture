import { useEffect, useState } from "react";
import { formatAccelerator } from "../../domain/settings";
import { btn } from "../components/styles";

/** Graba una combinación: clic, pulsar la combinación; `Esc` cancela y `Retroceso` restaura el valor por defecto. */
export function ShortcutRecorder({ value, onChange, label, fallback }: { value: string; onChange: (value: string) => void; label: string; fallback?: string }) {
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;
    // En captura y deteniendo la propagación: Esc/Retroceso no deben cerrar el diálogo ni disparar otros atajos.
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") return setRecording(false);
      if (e.key === "Backspace") {
        if (fallback !== undefined) onChange(fallback);
        return setRecording(false);
      }
      const accelerator = formatAccelerator(e);
      if (!accelerator) return; // solo modificadores o tecla no válida: sigue esperando
      onChange(accelerator);
      setRecording(false);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onChange, fallback]);

  return (
    <button
      type="button"
      className={`${btn.secondary} min-w-48 font-mono ${recording ? "ring-2 ring-t4s-blue" : ""}`}
      aria-label={`Grabar «${label}»`}
      onClick={() => setRecording((r) => !r)}
      onBlur={() => setRecording(false)}
    >
      {recording ? "Presiona la combinación…" : value}
    </button>
  );
}
