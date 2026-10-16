import { useCallback } from "react";
import { useWorkspace } from "../store/context";
import { readBytes, splitAccepted } from "./files";

export function useEvidenceIntake() {
  const intakeImages = useWorkspace((s) => s.intakeImages);
  const pushToast = useWorkspace((s) => s.pushToast);
  return useCallback(
    async (files: File[]) => {
      const { images, rejected } = splitAccepted(files);
      if (rejected) {
        pushToast(
          "error",
          rejected === 1
            ? "Se omitió 1 archivo: solo se aceptan imágenes PNG, JPEG o WebP."
            : `Se omitieron ${rejected} archivos: solo se aceptan imágenes PNG, JPEG o WebP.`,
        );
      }
      if (images.length) await intakeImages(await Promise.all(images.map(readBytes)));
    },
    [intakeImages, pushToast],
  );
}
