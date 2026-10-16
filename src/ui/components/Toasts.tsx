import { useEffect } from "react";
import { useWorkspace } from "../store/context";

/** Un toast con acciones («Abrir archivo») queda más tiempo para que se alcance a usar. */
const DURATION = { plain: 4000, actions: 10000 };

export function Toasts() {
  const toasts = useWorkspace((s) => s.toasts);
  const dismiss = useWorkspace((s) => s.dismissToast);

  useEffect(() => {
    if (toasts.length === 0) return;
    const first = toasts[0];
    const timer = setTimeout(() => dismiss(first.id), first.actions?.length ? DURATION.actions : DURATION.plain);
    return () => clearTimeout(timer);
  }, [toasts, dismiss]);

  return (
    <div role="status" aria-live="polite" className="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
      {toasts.map((t) => (
        <div key={t.id} className={`rounded-lg px-4 py-3 text-sm text-white shadow-lg ${t.kind === "error" ? "bg-red-700" : "bg-t4s-charcoal"}`}>
          <p>{t.message}</p>
          {t.actions?.length ? (
            <div className="mt-2 flex gap-3">
              {t.actions.map((a) => (
                <button
                  key={a.label}
                  type="button"
                  className="font-medium text-t4s-lime underline-offset-2 hover:underline"
                  onClick={() => {
                    a.run();
                    dismiss(t.id);
                  }}
                >
                  {a.label}
                </button>
              ))}
            </div>
          ) : null}
        </div>
      ))}
    </div>
  );
}
