import type { FieldDef, FieldType } from "../../domain/fields";
import { fixtureProblem, type Fixture } from "../../domain/fixtures";
import type { Template } from "../../domain/project";
import { AppError } from "../../ports/errors";
import type { FixtureStore } from "../../ports/repositories";

type Spec = [key: string, label: string, type: FieldType, width: number, extra?: Partial<FieldDef>];

const build = (templateId: string, specs: Spec[]): FieldDef[] =>
  specs.map(([key, label, type, width, extra], position) => ({
    id: `${templateId}-${key}`,
    key,
    label,
    type,
    options: [],
    required: false,
    clearOnDuplicate: false,
    width,
    position,
    archived: false,
    ...extra,
  }));

export const builtinTemplates: Template[] = [
  {
    id: "tpl-funcional",
    name: "Funcional",
    testType: "Funcional",
    builtin: true,
    fields: build("tpl-funcional", [
      ["entorno", "Entorno", "text", 21, { required: true }],
      ["modulo", "Módulo", "text", 22],
      ["menu", "Menú", "text", 24],
      ["submenu", "Submenú", "text", 24],
      ["funcionalidad", "Funcionalidad", "text", 28],
      ["datos_entrada", "Datos de entrada", "longtext", 48],
      ["resultado_esperado", "Resultado esperado", "longtext", 44],
      ["resultado_obtenido", "Resultado obtenido", "longtext", 44, { clearOnDuplicate: true }],
      ["fecha_ejecucion", "Fecha de ejecución", "date", 15, { clearOnDuplicate: true }],
      ["responsable", "Responsable", "text", 24],
      ["observaciones", "Observaciones", "longtext", 40, { clearOnDuplicate: true }],
    ]),
  },
  {
    id: "tpl-api",
    name: "API",
    testType: "API",
    builtin: true,
    fields: build("tpl-api", [
      ["entorno", "Entorno", "text", 21, { required: true }],
      ["endpoint", "Endpoint", "text", 40],
      ["metodo", "Método", "list", 12, { options: ["GET", "POST", "PUT", "PATCH", "DELETE"] }],
      ["codigo_http", "Código HTTP", "number", 12, { clearOnDuplicate: true }],
    ]),
  },
];

export interface MemoryFixtureStore extends FixtureStore {
  revealed: string[];
  copied: string[];
  removeForCase(caseId: string): void;
}

export function createMemoryFixtures(): MemoryFixtureStore {
  const fixtures: Fixture[] = [];
  const revealed: string[] = [];
  const copied: string[] = [];
  let seq = 0;

  return {
    revealed,
    copied,

    async list(caseId: string): Promise<Fixture[]> {
      return structuredClone(
        fixtures
          .filter((f) => f.caseId === caseId)
          .sort((a, b) => a.position - b.position),
      );
    },

    async add(caseId: string, fileName: string, bytes: Uint8Array): Promise<Fixture> {
      const problem = fixtureProblem(fileName, bytes.length);
      if (problem !== null) {
        throw new AppError("validation", problem);
      }
      const forCase = fixtures.filter((f) => f.caseId === caseId);
      if (forCase.length >= 50) {
        throw new AppError("validation", "El caso ya tiene 50 archivos de prueba.");
      }
      if (forCase.some((f) => f.fileName.toLowerCase() === fileName.toLowerCase())) {
        throw new AppError("validation", `El caso ya tiene un archivo llamado «${fileName}».`);
      }
      const fix: Fixture = {
        id: `fix-${++seq}`,
        caseId,
        fileName,
        size: bytes.length,
        sha256: `mock-sha256-${seq}`,
        note: "",
        position: forCase.length,
        addedAt: new Date().toISOString(),
      };
      fixtures.push(fix);
      return structuredClone(fix);
    },

    async remove(id: string): Promise<void> {
      const index = fixtures.findIndex((f) => f.id === id);
      if (index === -1) {
        throw new AppError("not_found", "El archivo de prueba no existe.");
      }
      const [removed] = fixtures.splice(index, 1);
      fixtures
        .filter((f) => f.caseId === removed.caseId)
        .forEach((f, idx) => {
          f.position = idx;
        });
    },

    async reveal(id: string): Promise<void> {
      const item = fixtures.find((f) => f.id === id);
      if (!item) {
        throw new AppError("not_found", "El archivo de prueba no existe.");
      }
      revealed.push(id);
    },

    async copyPath(id: string): Promise<void> {
      const item = fixtures.find((f) => f.id === id);
      if (!item) {
        throw new AppError("not_found", "El archivo de prueba no existe.");
      }
      copied.push(id);
    },

    removeForCase(caseId: string): void {
      for (let i = fixtures.length - 1; i >= 0; i--) {
        if (fixtures[i].caseId === caseId) {
          fixtures.splice(i, 1);
        }
      }
    },
  };
}
