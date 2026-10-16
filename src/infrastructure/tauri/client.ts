import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import type { z } from "zod";
import { AppError } from "../../ports/errors";
import { appErrorSchema } from "./dto";

async function invokeCore(command: string, args: InvokeArgs): Promise<unknown> {
  try {
    return await invoke(command, args);
  } catch (error) {
    const parsed = appErrorSchema.safeParse(error);
    if (parsed.success) throw new AppError(parsed.data.code, parsed.data.message, parsed.data.detail);
    throw new AppError("internal", "No se pudo comunicar con el núcleo de Qapture.", String(error));
  }
}

export async function call<S extends z.ZodType>(command: string, args: InvokeArgs, schema: S): Promise<z.infer<S>> {
  const raw = await invokeCore(command, args);
  const result = schema.safeParse(raw);
  if (!result.success) {
    throw new AppError("contract", "La respuesta del núcleo no tiene el formato esperado.", result.error.message);
  }
  return result.data;
}

/** Para comandos que devuelven `tauri::ipc::Response` (bytes crudos). */
export async function callBytes(command: string, args: InvokeArgs): Promise<Uint8Array> {
  const raw = await invokeCore(command, args);
  if (raw instanceof ArrayBuffer) return new Uint8Array(raw);
  if (raw instanceof Uint8Array) return raw;
  throw new AppError("contract", "La respuesta del núcleo no tiene el formato esperado.", "se esperaba contenido binario");
}
