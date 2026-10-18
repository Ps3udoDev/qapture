import type { StoreApi } from "zustand/vanilla";
import { progressPercent, SAVE_FIRST_MESSAGE, shouldPrompt, upToDateMessage, type UpdateInfo } from "../../domain/update";
import type { Repositories } from "../../ports/repositories";
import type { SliceHelpers } from "./evidence-slice";
import type { WorkspaceState } from "./workspace";

export interface UpdateState {
  /** Versión pendiente de instalar, si la hay (se conserva aunque se haya pospuesto). */
  available: UpdateInfo | null;
  /** Versión que la persona pospuso con «Después»; solo una versión más nueva vuelve a abrir el diálogo. */
  dismissed: string | null;
  status: "idle" | "checking" | "downloading";
  /** Porcentaje 0-100 de la descarga; `null` si aún no empieza o no se conoce el total. */
  progress: number | null;
}

export interface UpdateSlice {
  update: UpdateState;
  /** `manual`: avisa si está al día y muestra el error; la automática falla en silencio. */
  checkForUpdates(manual: boolean): Promise<void>;
  installUpdate(): Promise<void>;
  dismissUpdate(): void;
  /** Suscribe `update:available` y `update:progress`; devuelve la función que cancela ambas. */
  listenUpdates(): Promise<() => void>;
}

type Set = StoreApi<WorkspaceState>["setState"];
type Get = StoreApi<WorkspaceState>["getState"];

/** El diálogo está abierto: hay una versión pendiente y no es una pospuesta (ni anterior). */
export const isUpdatePromptOpen = (s: Pick<WorkspaceState, "update">): boolean =>
  s.update.available !== null && shouldPrompt(s.update.available.version, s.update.dismissed);

/** Hay una versión pendiente que se pospuso: el indicador de Ajustes sigue visible. */
export const hasPostponedUpdate = (s: Pick<WorkspaceState, "update">): boolean => s.update.available !== null && !isUpdatePromptOpen(s);

export function createUpdateSlice(repos: Repositories, set: Set, get: Get, h: SliceHelpers): UpdateSlice {
  let installing = false;
  const patch = (change: Partial<UpdateState>) => set((s) => ({ update: { ...s.update, ...change } }));

  return {
    update: { available: null, dismissed: null, status: "idle", progress: null },

    async checkForUpdates(manual) {
      if (get().update.status !== "idle") return;
      patch({ status: "checking" });
      try {
        const info = await repos.updater.check();
        patch({ available: info });
        if (!info && manual) get().pushToast("success", upToDateMessage(get().settings?.appVersion ?? ""));
      } catch (error) {
        // Sin red o sin `latest.json`: solo la revisión manual lo cuenta; la automática lo anota en consola.
        if (manual) h.report(error);
        else console.warn("No se pudo buscar actualizaciones.", error);
      } finally {
        patch({ status: "idle" });
      }
    },

    async installUpdate() {
      if (installing) return;
      // Reiniciar borraría `_staging`: una captura abierta o en cola se perdería.
      if (get().editor || get().editorQueue.length > 0) {
        get().pushToast("error", SAVE_FIRST_MESSAGE);
        return;
      }
      installing = true;
      patch({ status: "downloading", progress: null });
      try {
        await repos.updater.install();
      } catch (error) {
        h.report(error);
      } finally {
        installing = false;
        patch({ status: "idle", progress: null });
      }
    },

    dismissUpdate() {
      const available = get().update.available;
      if (available) patch({ dismissed: available.version });
    },

    async listenUpdates() {
      // `onAvailable` puede repetir la misma versión (revisión cada 6 h, clic en la bandeja): el diálogo
      // solo se reabre si no está pospuesta; el indicador de Ajustes y «Acerca de» son el camino de vuelta.
      const offAvailable = await repos.updater.onAvailable((info) => patch({ available: info }));
      try {
        const offProgress = await repos.updater.onProgress((p) => patch({ status: "downloading", progress: progressPercent(p.downloaded, p.total) }));
        return () => {
          offAvailable();
          offProgress();
        };
      } catch (error) {
        offAvailable();
        throw error;
      }
    },
  };
}
