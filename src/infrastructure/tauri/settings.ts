import { listen } from "@tauri-apps/api/event";
import { z } from "zod";
import type { SettingsStore } from "../../ports/settings";
import { call } from "./client";
import { settingsSnapshotSchema } from "./dto";

export const NEW_CASE_EVENT = "shortcut:new-case";

export function tauriSettings(): SettingsStore {
  return {
    load: () => call("get_settings", {}, settingsSnapshotSchema),
    save: (settings) => call("save_settings", { settings }, settingsSnapshotSchema),
    pickExportFolder: () => call("pick_export_folder", {}, z.string().nullable()),
    openDataFolder: async () => {
      await call("open_data_folder", {}, z.null());
    },
    hideWindow: async () => {
      await call("hide_main_window", {}, z.null());
    },
    openLink: async (target) => {
      await call("open_link", { target }, z.null());
    },
    onNewCase: (handler) => listen(NEW_CASE_EVENT, () => handler()),
  };
}
