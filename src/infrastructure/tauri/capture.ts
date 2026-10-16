import { listen } from "@tauri-apps/api/event";
import { z } from "zod";
import type { OverlayApi, ScreenCapture } from "../../ports/capture";
import { call, callBytes } from "./client";
import { capturedEventSchema } from "./dto";

const nothing = z.null();

export const CAPTURED_EVENT = "evidence:captured";

export function tauriScreenCapture(): ScreenCapture {
  return {
    start: async () => {
      await call("start_capture", {}, nothing);
    },
    onCaptured: (handler) =>
      listen<unknown>(CAPTURED_EVENT, (event) => {
        const parsed = capturedEventSchema.safeParse(event.payload);
        if (parsed.success) handler(parsed.data.tmpPath);
      }),
  };
}

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
