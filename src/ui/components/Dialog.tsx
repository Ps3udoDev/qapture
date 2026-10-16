import { X } from "lucide-react";
import { useEffect, type ReactNode } from "react";
import { iconButton } from "./styles";

export function Dialog({ title, kicker, wide = false, onClose, children }: {
  title: string;
  kicker?: string;
  wide?: boolean;
  onClose: () => void;
  children: ReactNode;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-40 grid place-items-center bg-t4s-charcoal/40 p-4">
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={`flex max-h-[90vh] w-full flex-col rounded-xl bg-white shadow-xl ${wide ? "max-w-4xl" : "max-w-xl"}`}
      >
        <header className="flex items-start justify-between border-b border-t4s-line px-6 py-4">
          <div>
            {kicker && <p className="text-xs font-semibold uppercase tracking-wide text-t4s-blue">{kicker}</p>}
            <h2 className="text-lg font-semibold">{title}</h2>
          </div>
          <button type="button" onClick={onClose} aria-label="Cerrar" className={iconButton}>
            <X size={18} />
          </button>
        </header>
        <div className="overflow-y-auto px-6 py-4">{children}</div>
      </div>
    </div>
  );
}

export function DialogFooter({ children }: { children: ReactNode }) {
  return <div className="col-span-full mt-2 flex justify-end gap-2 border-t border-t4s-line pt-4">{children}</div>;
}
