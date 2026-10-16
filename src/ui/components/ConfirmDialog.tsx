import { useState } from "react";
import { Dialog, DialogFooter } from "./Dialog";
import { FormField } from "./FormField";
import { btn, inputCls } from "./styles";

export function ConfirmDialog({ title, message, confirmLabel, requireText, onCancel, onConfirm }: {
  title: string;
  message: string;
  confirmLabel: string;
  requireText?: string;
  onCancel: () => void;
  onConfirm: () => void | Promise<void>;
}) {
  const [typed, setTyped] = useState("");
  const allowed = !requireText || typed.trim() === requireText;
  return (
    <Dialog title={title} onClose={onCancel}>
      <p className="text-sm">{message}</p>
      {requireText && (
        <FormField label={`Escribe «${requireText}» para confirmar`} htmlFor="confirm-text" className="mt-4">
          <input id="confirm-text" className={inputCls} value={typed} onChange={(e) => setTyped(e.target.value)} />
        </FormField>
      )}
      <DialogFooter>
        <button type="button" className={btn.secondary} onClick={onCancel}>
          Cancelar
        </button>
        <button type="button" className={btn.danger} disabled={!allowed} onClick={() => void onConfirm()}>
          {confirmLabel}
        </button>
      </DialogFooter>
    </Dialog>
  );
}
