export interface AgentClientConfig {
  bridgePath: string;
  listening: boolean;
  claudeCode: string;
  codex: string;
  json: string;
}
export interface Activity {
  id: string;
  at: string;
  client: string;
  tool: string;
  projectId: string | null;
  caseId: string | null;
  ok: boolean;
  detail: string;
}
export interface AgentDataChanged {
  projectId: string | null;
  caseId: string | null;
  summary: string;
}
export const RECOMMENDED = { read: true, write: true } as const;
export const AGENT_GROUPS = [
  {
    id: "read",
    label: "Lectura",
    description: "Consultar proyectos, casos, campos y evidencias.",
    risk: "Puede exponer datos del cliente al agente.",
    tools: [
      "list_templates",
      "list_projects",
      "get_project",
      "list_cases",
      "get_case",
      "get_evidence_image",
    ],
  },
  {
    id: "write",
    label: "Escritura",
    description:
      "Crear proyectos desde plantillas existentes y casos, cambiar estados y valores, adjuntar evidencias.",
    risk: "Modifica datos. Nunca borra.",
    tools: [
      "create_project",
      "create_case",
      "update_case",
      "add_evidence",
      "update_evidence_description",
    ],
  },
  {
    id: "capture",
    label: "Captura de pantalla",
    description: "Capturar un monitor o una ventana visible.",
    risk: "Puede capturar información sensible. Pide confirmación en cada uso.",
    tools: ["capture_screen", "list_windows"],
  },
  {
    id: "export",
    label: "Exportar",
    description: "Crear .xlsx y .qapture en la carpeta elegida.",
    risk: "Genera archivos con datos del proyecto. Pide confirmación en cada uso.",
    tools: ["export_excel", "export_package"],
  },
] as const;
export function summarizeAgentChanges(summaries: string[]): string {
  return summaries.length === 1
    ? summaries[0]
    : `Un agente hizo ${summaries.length} cambios`;
}
