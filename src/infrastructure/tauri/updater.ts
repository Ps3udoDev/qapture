import { listen } from "@tauri-apps/api/event";
import { z } from "zod";
import type { Updater } from "../../ports/updater";
import { call } from "./client";
import { updateInfoSchema, updateProgressSchema } from "./dto";

export const UPDATE_AVAILABLE_EVENT = "update:available";
export const UPDATE_PROGRESS_EVENT = "update:progress";

export function tauriUpdater(): Updater {
  return {
    check: () => call("check_update", {}, updateInfoSchema.nullable()),
    install: async () => {
      await call("install_update", {}, z.null());
    },
    onAvailable: (handler) =>
      listen<unknown>(UPDATE_AVAILABLE_EVENT, (event) => {
        const parsed = updateInfoSchema.safeParse(event.payload);
        if (parsed.success) handler(parsed.data);
        else console.warn(`Evento ${UPDATE_AVAILABLE_EVENT} con formato inesperado.`, parsed.error.message);
      }),
    onProgress: (handler) =>
      listen<unknown>(UPDATE_PROGRESS_EVENT, (event) => {
        const parsed = updateProgressSchema.safeParse(event.payload);
        if (parsed.success) handler(parsed.data);
        else console.warn(`Evento ${UPDATE_PROGRESS_EVENT} con formato inesperado.`, parsed.error.message);
      }),
  };
}
