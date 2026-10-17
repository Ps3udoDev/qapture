import { Database } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";

function MenuItem({ children, disabled = false, onClick }: { children: ReactNode; disabled?: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      role="menuitem"
      disabled={disabled}
      onClick={onClick}
      className="block w-full px-4 py-2 text-left text-sm hover:bg-t4s-cream disabled:cursor-not-allowed disabled:opacity-50"
    >
      {children}
    </button>
  );
}

/** UC-18 / UC-19 / UC-20. F6 lleva estas acciones también a Settings › Datos. */
export function DataMenu({ onBackups }: { onBackups: () => void }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const project = useWorkspace((s) => s.project);
  const busy = useWorkspace((s) => s.dataBusy);
  const importLegacy = useWorkspace((s) => s.importLegacy);
  const importPackage = useWorkspace((s) => s.importPackage);
  const exportPackage = useWorkspace((s) => s.exportPackage);

  const items = () => Array.from(menu.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)') ?? []);

  useEffect(() => {
    if (open) items()[0]?.focus();
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const onPointer = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      setOpen(false);
      trigger.current?.focus();
    };
    document.addEventListener("mousedown", onPointer);
    window.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onPointer);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const onMenuKey = (e: ReactKeyboardEvent) => {
    const list = items();
    if (list.length === 0) return;
    const at = list.indexOf(document.activeElement as HTMLButtonElement);
    let next: number;
    if (e.key === "ArrowDown") next = (at + 1) % list.length;
    else if (e.key === "ArrowUp") next = (at <= 0 ? list.length : at) - 1;
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = list.length - 1;
    else return;
    e.preventDefault();
    list[next].focus();
  };

  const run = (action: () => void) => () => {
    setOpen(false);
    action();
  };

  return (
    <div ref={ref} className="relative">
      <button ref={trigger} type="button" className={btn.secondary} aria-haspopup="menu" aria-expanded={open} disabled={busy} onClick={() => setOpen((o) => !o)}>
        <Database size={16} /> Datos
      </button>
      {open && (
        <div ref={menu} role="menu" aria-label="Datos" onKeyDown={onMenuKey} className="absolute right-0 z-30 mt-1 w-64 rounded-lg border border-t4s-line bg-white py-1 shadow-lg">
          <MenuItem onClick={run(() => void importLegacy())}>Importar Bitácora v1…</MenuItem>
          <MenuItem onClick={run(() => void importPackage())}>Importar paquete .qapture…</MenuItem>
          <MenuItem disabled={!project} onClick={run(() => void exportPackage())}>
            Exportar paquete .qapture…
          </MenuItem>
          <hr className="my-1 border-t4s-line" />
          <MenuItem onClick={run(onBackups)}>Respaldos…</MenuItem>
        </div>
      )}
    </div>
  );
}
