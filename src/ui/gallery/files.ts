import { isAcceptedImage } from "../../domain/evidence";

export function filesFrom(data: DataTransfer | null): File[] {
  if (!data) return [];
  const files = Array.from(data.files ?? []);
  if (files.length) return files;
  return Array.from(data.items ?? [])
    .filter((item) => item.kind === "file")
    .map((item) => item.getAsFile())
    .filter((f): f is File => f !== null);
}

export const hasFiles = (data: DataTransfer | null) => !!data && Array.from(data.types ?? []).includes("Files");

export function splitAccepted(files: File[]) {
  const images = files.filter(isAcceptedImage);
  return { images, rejected: files.length - images.length };
}

export async function readBytes(file: Blob): Promise<Uint8Array> {
  if (typeof file.arrayBuffer === "function") return new Uint8Array(await file.arrayBuffer());
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(new Uint8Array(reader.result as ArrayBuffer));
    reader.onerror = () => reject(reader.error);
    reader.readAsArrayBuffer(file);
  });
}
