import type { ImportedProject } from "../../domain/data";
import { columnLetter, fold, type GuionImportInput, type GuionInspection, type GuionPreview, type ImportProfile, type ProfileDefinition } from "../../domain/guion";
import { STATUSES, type Status } from "../../domain/status";
import { AppError } from "../../ports/errors";
import type { GuionImporter } from "../../ports/guion";

export interface MemoryGuion extends GuionImporter {
  mode: "ok" | "cancel";
  calls: string[];
  inspection: GuionInspection;
  profiles: ImportProfile[];
}
const copy = <T>(value: T): T => structuredClone(value);
const fail = (message: string): never => { throw new AppError("validation", message); };
const HEADERS = ["Módulo", "SubMódulo", "ID", "Opción", "RN", "Escenario de Prueba", "Resultado Esperado", "Criticidad", "Fecha de Prueba", "Version", "Status", "Nombre del tester", "Condiciones"];

export function createMemoryGuion(deps: { importProject(name: string, cases: number): Promise<ImportedProject> }): MemoryGuion {
  let token: string | null = null;
  let sequence = 0;
  const inspect: GuionInspection = {
    token: "guion-0", fileName: "guion_sintetico.xlsx", suggestedProfileId: null,
    sheets: [{ name: "Guión de Pruebas", suggestedHeaderRow: 15, cover: { executionDate: "D5", responsible: "D6", observations: "D8" }, rows: [
      ...Array.from({ length: 14 }, () => [] as string[]), HEADERS,
      ...Array.from({ length: 3 }, (_, i) => ["Aplicación", "Módulo", `CP0${i + 1}`, "Proceso", "RN01", `Escenario ${i + 1}`, "Resultado", i % 2 ? "Importante" : "Crítico", "", "1.0", "", "", ""]),
    ] }],
  };
  const ensureToken = (value: string) => { if (!token || value !== token) fail("Vuelve a elegir el archivo del guion."); };
  const plan = (definition: ProfileDefinition): GuionPreview => {
    const result: GuionPreview = { cases: [], fields: [], errors: [], warnings: [] };
    const sheet = guion.inspection.sheets.find((s) => s.name === definition.sheet);
    const codes = definition.columns.filter((c) => c.target.kind === "code");
    if (!codes.length) result.errors.push("Falta asignar la columna del código del caso.");
    if (codes.length > 1) result.errors.push("Solo una columna puede ser el código.");
    if (!Number.isInteger(definition.headerRow) || definition.headerRow < 1) result.errors.push("La fila de encabezados debe ser 1 o mayor.");
    if (!sheet) result.errors.push(`El guion no tiene la hoja «${definition.sheet}».`);
    if (result.errors.length || !sheet) return result;
    const at = (letter: string) => {
      let n = 0;
      for (const c of letter) n = n * 26 + c.charCodeAt(0) - 64;
      return n - 1;
    };
    const title = definition.columns.find((c) => c.target.kind === "title");
    const status = definition.columns.find((c) => c.target.kind === "status");
    for (const { target } of definition.columns) if (target.kind === "field") result.fields.push({ key: target.key, label: target.label, type: target.type, options: [...target.options] });
    if (Object.values(definition.statusMap.import).includes("Aprobado+condicionado")) result.fields.push({ key: "condicionado", label: "Condicionado", type: "boolean", options: [] });
    const missing: number[] = [];
    const counts = new Map<string, number>();
    sheet.rows.slice(definition.headerRow).forEach((row, index) => {
      if (definition.columns.every((c) => !(row[at(c.column)] ?? "").trim())) return;
      const number = definition.headerRow + index + 1;
      const code = (row[at(codes[0].column)] ?? "").trim();
      if (!code) missing.push(number); else counts.set(code, (counts.get(code) ?? 0) + 1);
      const raw = status ? row[at(status.column)] ?? "" : "";
      const mapped = Object.entries(definition.statusMap.import).find(([key]) => fold(key) === fold(raw))?.[1];
      const conditioned = mapped === "Aprobado+condicionado";
      const value: Status = conditioned ? "Aprobado" : STATUSES.includes(mapped as Status) ? mapped as Status : "No ejecutado";
      if (status && mapped === undefined) result.warnings.push(`${code}: «${raw}» no es un estado del perfil; el caso entra como «No ejecutado».`);
      result.cases.push({ row: number, code, title: (title ? row[at(title.column)] : "") || code, status: value, conditioned });
    });
    if (missing.length) result.errors.push(`Hay filas sin código: ${missing.join(", ")}.`);
    const duplicates = [...counts].filter(([, n]) => n > 1).map(([code]) => code);
    if (duplicates.length) result.errors.push(`Hay códigos repetidos: ${duplicates.join(", ")}.`);
    if (!result.cases.length) result.errors.push("El guion no tiene casos debajo de la fila de encabezados.");
    if (result.cases.length > 10_000) result.errors.push("El guion supera 10000 casos.");
    return result;
  };
  const guion: MemoryGuion = {
    mode: "ok", calls: [], inspection: inspect, profiles: [],
    async inspect() {
      guion.calls.push("inspect");
      if (guion.mode === "cancel") return null;
      token = `guion-${++sequence}`;
      guion.inspection.token = token;
      const suggested = guion.profiles.find(({ definition }) => {
        const sheet = guion.inspection.sheets.find((s) => s.name === definition.sheet);
        return sheet && definition.columns.length > 0 && definition.columns.every((c) => {
          const row = sheet.rows[definition.headerRow - 1] ?? [];
          return row.some((header, i) => columnLetter(i) === c.column && fold(header) === fold(c.header));
        });
      });
      guion.inspection.suggestedProfileId = suggested?.id ?? null;
      return copy(guion.inspection);
    },
    async preview(value, definition) { guion.calls.push("preview"); ensureToken(value); return plan(definition); },
    async import(input: GuionImportInput) {
      guion.calls.push("import");
      ensureToken(input.token);
      const preview = plan(input.profile);
      if (preview.errors.length) fail(`No se importó nada. Revisa estos problemas:\n${preview.errors.map((e) => `• ${e}`).join("\n")}`);
      const name = input.projectName.trim();
      if (!name) fail("«Nombre del proyecto» es obligatorio.");
      if ([...name].length > 150) fail("«Nombre del proyecto» supera 150 caracteres.");
      const profileName = input.saveProfileAs?.trim();
      if (input.saveProfileAs !== null && !profileName) fail("«Nombre del perfil» es obligatorio.");
      if (profileName && [...profileName].length > 100) fail("«Nombre del perfil» supera 100 caracteres.");
      if (profileName && guion.profiles.some((p) => p.name.toLowerCase() === profileName.toLowerCase())) fail(`Ya existe un perfil llamado «${profileName}».`);
      const project = await deps.importProject(name, preview.cases.length);
      if (profileName) {
        const stamp = new Date().toISOString();
        guion.profiles.push({ id: `profile-${++sequence}`, name: profileName, definition: copy(input.profile), createdAt: stamp, updatedAt: stamp });
      }
      token = null;
      return { projects: [project], cases: preview.cases.length, evidence: 0, warnings: preview.warnings };
    },
    async listProfiles() { guion.calls.push("listProfiles"); return copy([...guion.profiles].sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()))); },
    async deleteProfile(id) {
      guion.calls.push(`deleteProfile:${id}`);
      if (!guion.profiles.some((p) => p.id === id)) throw new AppError("not_found", "El perfil no existe.");
      guion.profiles = guion.profiles.filter((p) => p.id !== id);
    },
  };
  return guion;
}
