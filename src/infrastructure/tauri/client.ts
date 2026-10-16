import { invoke } from "@tauri-apps/api/core";
import type { z } from "zod";
import { AppError } from "../../ports/errors";
import { appErrorSchema } from "./dto";

export async function call<S extends z.ZodType>(command: string, args: Record<string, unknown>, schema: S): Promise<z.infer<S>> {
  let raw: unknown;
  try {
    raw = await invoke(command, args);
  } catch (error) {
    const parsed = appErrorSchema.safeParse(error);
    if (parsed.success) throw new AppError(parsed.data.code, parsed.data.message, parsed.data.detail);
    throw new AppError("internal", "No se pudo comunicar con el núcleo de Qapture.", String(error));
  }
  const result = schema.safeParse(raw);
  if (!result.success) {
    throw new AppError("contract", "La respuesta del núcleo no tiene el formato esperado.", result.error.message);
  }
  return result.data;
}
