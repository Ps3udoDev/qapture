import type { FieldDef, FieldType } from "../../domain/fields";
import type { Template } from "../../domain/project";

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
