import { z } from "zod";
import type { OverlayApi } from "../../ports/capture";
import { call, callBytes } from "./client";

const nothing = z.null();

export function tauriOverlayApi(): OverlayApi {
  return {
    loadFrame: () => callBytes("capture_frame", {}),
    ready: async () => {
      await call("overlay_ready", {}, nothing);
    },
    finish: async (selection) => {
      await call("finish_capture", { selection }, nothing);
    },
    cancel: async () => {
      await call("cancel_capture", {}, nothing);
    },
  };
}
