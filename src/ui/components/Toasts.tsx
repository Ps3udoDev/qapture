import { useEffect } from "react";
import { useWorkspace } from "../store/context";

export function Toasts() {
  const toasts = useWorkspace((s) => s.toasts);
  const dismiss = useWorkspace((s) => s.dismissToast);

  useEffect(() => {
    if (toasts.length === 0) return;
    const timer = setTimeout(() => dismiss(toasts[0].id), 4000);
    return () => clearTimeout(timer);
  }, [toasts, dismiss]);

  return (
    <div role="status" aria-live="polite" className="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
      {toasts.map((t) => (
        <div key={t.id} className={`rounded-lg px-4 py-3 text-sm text-white shadow-lg ${t.kind === "error" ? "bg-red-700" : "bg-t4s-charcoal"}`}>
          {t.message}
        </div>
      ))}
    </div>
  );
}
