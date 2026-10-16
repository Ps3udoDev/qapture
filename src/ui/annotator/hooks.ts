import { useEffect, useRef, useState, type RefObject } from "react";

export function useBoxSize(ref: RefObject<HTMLElement | null>) {
  const [size, setSize] = useState({ width: 0, height: 0 });
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const update = () => setSize({ width: el.clientWidth, height: el.clientHeight });
    update();
    const observer = new ResizeObserver(update);
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref]);
  return size;
}

/** Carga el original por IPC y lo expone como `blob:` (mismo origen: el canvas no queda "tainted" al exportar). */
export function useLoadedImage(path: string, load: (path: string) => Promise<Uint8Array>) {
  const [state, setState] = useState<{ image: HTMLImageElement | null; error: string | null }>({ image: null, error: null });
  const loadRef = useRef(load);
  loadRef.current = load;
  useEffect(() => {
    let cancelled = false;
    let url: string | null = null;
    setState({ image: null, error: null });
    loadRef.current(path)
      .then((bytes) => {
        if (cancelled) return;
        url = URL.createObjectURL(new Blob([bytes as BlobPart], { type: "image/png" }));
        const img = new window.Image();
        img.onload = () => !cancelled && setState({ image: img, error: null });
        img.onerror = () => !cancelled && setState({ image: null, error: "No se pudo abrir la imagen." });
        img.src = url;
      })
      .catch(() => !cancelled && setState({ image: null, error: "No se pudo abrir la imagen." }));
    return () => {
      cancelled = true;
      if (url) URL.revokeObjectURL(url);
    };
  }, [path]);
  return state;
}
