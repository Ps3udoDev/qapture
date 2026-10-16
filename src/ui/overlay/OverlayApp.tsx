import { useEffect, useRef, useState, type PointerEvent } from "react";
import { isUsable, rectFrom, sizeLabel, toSelection, type Point, type Rect } from "../../domain/capture";
import type { OverlayApi } from "../../ports/capture";
import { AppError } from "../../ports/errors";

const HINT = "Arrastra para seleccionar · Enter: pantalla completa · Esc: cancelar";
const DIM = "rgba(0, 0, 0, 0.45)";

const viewport = () => ({ width: window.innerWidth, height: window.innerHeight });

/** Una ventana por monitor: imagen congelada a pantalla completa, oscurecida, con la selección en claro. */
export function OverlayApp({ api }: { api: OverlayApi }) {
  const [url, setUrl] = useState<string | null>(null);
  const [start, setStart] = useState<Point | null>(null);
  const [rect, setRect] = useState<Rect | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const busy = useRef(false);

  useEffect(() => {
    let alive = true;
    let created: string | null = null;
    api
      .loadFrame()
      .then((bytes) => {
        if (!alive) return;
        created = URL.createObjectURL(new Blob([bytes as BlobPart], { type: "image/png" }));
        setUrl(created);
      })
      .catch(() => {
        if (alive) void api.cancel().catch(() => {});
      });
    return () => {
      alive = false;
      if (created) URL.revokeObjectURL(created);
    };
  }, [api]);

  const send = useRef(async (run: () => Promise<void>) => {
    if (busy.current) return;
    busy.current = true;
    try {
      await run();
    } catch (error) {
      // La sesión sigue abierta en Rust: se puede volver a seleccionar.
      busy.current = false;
      setRect(null);
      setMessage(error instanceof AppError ? error.message : "No se pudo completar la captura.");
    }
  }).current;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        void send(() => api.cancel());
      } else if (e.key === "Enter") {
        e.preventDefault();
        void send(() => api.finish(null));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [api, send]);

  const point = (e: PointerEvent): Point => ({ x: e.clientX, y: e.clientY });

  return (
    <div
      data-testid="overlay-surface"
      className="fixed inset-0 cursor-crosshair select-none overflow-hidden bg-black"
      onContextMenu={(e) => e.preventDefault()}
      onPointerDown={(e) => {
        if (e.button !== 0 || busy.current) return;
        try {
          // Sigue recibiendo el arrastre aunque el puntero salga de la ventana.
          e.currentTarget.setPointerCapture(e.pointerId);
        } catch {
          // jsdom no implementa la captura de puntero.
        }
        const p = point(e);
        setStart(p);
        setRect(rectFrom(p, p, viewport()));
        setMessage(null);
      }}
      onPointerMove={(e) => {
        if (start) setRect(rectFrom(start, point(e), viewport()));
      }}
      onPointerUp={(e) => {
        if (!start) return;
        const final = rectFrom(start, point(e), viewport());
        setStart(null);
        if (!isUsable(final)) {
          setRect(null);
          return;
        }
        setRect(final);
        void send(() => api.finish(toSelection(final, viewport())));
      }}
    >
      {url && (
        <img
          data-testid="frame"
          src={url}
          alt=""
          draggable={false}
          className="pointer-events-none absolute inset-0 h-full w-full"
          onLoad={() => void api.ready().catch(() => {})}
          // Una imagen que no se decodifica dejaría la captura colgada: se cancela.
          onError={() => void api.cancel().catch(() => {})}
        />
      )}
      {rect ? (
        <div
          data-testid="selection"
          className="pointer-events-none absolute border-2 border-t4s-lime"
          style={{ left: rect.x, top: rect.y, width: rect.width, height: rect.height, boxShadow: `0 0 0 100000px ${DIM}` }}
        >
          <span
            className="absolute left-0 whitespace-nowrap rounded bg-t4s-charcoal px-2 py-0.5 text-xs text-white"
            style={{ top: rect.y >= 28 ? -28 : 4 }}
          >
            {sizeLabel(rect, window.devicePixelRatio || 1)}
          </span>
        </div>
      ) : (
        <div className="pointer-events-none absolute inset-0" style={{ background: DIM }} />
      )}
      <p
        role="status"
        className="pointer-events-none absolute left-1/2 top-4 -translate-x-1/2 rounded-full bg-t4s-charcoal/90 px-4 py-2 text-sm text-white shadow-lg"
      >
        {message ?? HINT}
      </p>
    </div>
  );
}
