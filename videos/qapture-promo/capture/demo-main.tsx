// Arnés de captura para el video: monta la UI real de Qapture sobre los repositorios en memoria
// con un proyecto de ejemplo. Solo lo usa capture/shoot.mjs; no forma parte de la app.
import "../../../src/ui/theme/tokens.css";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "../../../src/App";
import { createMemoryRepositories } from "../../../src/infrastructure/memory/repositories";
import { createWorkspaceStore } from "../../../src/ui/store/workspace";
import type { Status } from "../../../src/domain/status";

const scene = new URLSearchParams(location.search).get("scene") ?? "workspace";

async function main() {
  const repos = createMemoryRepositories();
  const sut = new Uint8Array(await (await fetch("./sut.png")).arrayBuffer());

  const urls = new Map<string, string>();
  const readImage = repos.evidence.readImage.bind(repos.evidence);
  repos.evidence.imageUrl = (path) => urls.get(path) ?? "";
  const remember = async (path: string) => {
    const bytes = await readImage(path);
    urls.set(path, URL.createObjectURL(new Blob([bytes as BlobPart], { type: "image/png" })));
  };

  const project = await repos.projects.create({
    meta: {
      name: "Portal Andes — Release 2.4",
      client: "Banco Andes",
      testType: "Funcional",
      releaseVersion: "2.4.0",
      startDate: "2026-10-12",
      endDate: "2026-10-23",
    },
    templateId: "tpl-funcional",
  });

  const rows: [string, Status, string, string, string][] = [
    ["Login con credenciales válidas", "Aprobado", "Autenticación", "Inicio de sesión", "Acceso al panel principal"],
    ["Transferencia a tercero", "Fallido", "Transferencias", "Nueva transferencia", "Transferencia confirmada con comprobante"],
    ["Consulta de saldo", "Aprobado", "Cuentas", "Resumen", "Saldo actualizado en tiempo real"],
    ["Pago de tarjeta de crédito", "En ejecución", "Pagos", "Tarjetas", "Pago aplicado al estado de cuenta"],
    ["Registro de beneficiario", "Por documentar", "Transferencias", "Beneficiarios", "Beneficiario disponible para transferir"],
    ["Bloqueo por intentos fallidos", "Aprobado", "Autenticación", "Seguridad", "Cuenta bloqueada tras 3 intentos"],
    ["Descarga de estado de cuenta", "Bloqueado", "Cuentas", "Historial", "PDF generado con el período elegido"],
    ["Transferencia programada", "No ejecutado", "Transferencias", "Programadas", "Transferencia ejecutada en la fecha"],
  ];
  const fieldId = new Map(project.fields.map((f) => [f.key, f.id]));
  const byId = (values: Record<string, string>) => Object.fromEntries(Object.entries(values).map(([k, v]) => [fieldId.get(k) ?? k, v]));
  const cases = [];
  for (const [title, status, modulo, menu, esperado] of rows) {
    cases.push(
      await repos.cases.create(project.id, {
        title,
        status,
        values: byId({
          entorno: "QA",
          modulo,
          menu,
          funcionalidad: title,
          resultado_esperado: esperado,
          responsable: "María Torres",
          fecha_ejecucion: "2026-10-18",
        }),
      }),
    );
  }

  const target = cases[1];
  const annotated = JSON.stringify({
    version: 1,
    crop: null,
    shapes: [
      { id: "s1", kind: "rect", x: 548, y: 88, width: 478, height: 82, color: "#E02424", strokeWidth: 5 },
      { id: "s2", kind: "marker", x: 1050, y: 128, color: "#E02424", radius: 22 },
      { id: "s3", kind: "arrow", points: [610, 640, 440, 548], color: "#2B629E", strokeWidth: 6 },
      { id: "s4", kind: "marker", x: 636, y: 652, color: "#2B629E", radius: 22 },
      { id: "s5", kind: "text", x: 668, y: 634, text: "Clic en Transferir", color: "#2B629E", fontSize: 26 },
      { id: "s6", kind: "blur", x: 1018, y: 0, width: 262, height: 64 },
    ],
  });
  for (const [i, description] of ["Formulario con datos válidos", "Error 500 al confirmar la transferencia"].entries()) {
    const staged = await repos.evidence.stage(sut);
    const ev = await repos.evidence.add({ caseId: target.id, stagedPath: staged.path, description, annotations: i === 1 ? annotated : JSON.stringify({ version: 1, crop: null, shapes: [] }) }, sut);
    await remember(ev.renderedPath);
  }

  const store = createWorkspaceStore(repos, {
    getLastProject: () => project.id,
    setLastProject: () => {},
    getTester: () => "María Torres",
  });

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App store={store} />
    </React.StrictMode>,
  );

  const waitReady = async () => {
    while (!store.getState().ready) await new Promise((r) => setTimeout(r, 50));
  };
  await waitReady();
  store.getState().selectCase(target.id);
  await new Promise((r) => setTimeout(r, 300));

  if (scene === "editor-new") {
    const staged = await repos.evidence.stage(sut);
    store.getState().openEvidenceEditor({ imagePath: staged.path, caseId: target.id });
  } else if (scene === "editor-edit") {
    const evidence = store.getState().evidence.at(-1);
    if (evidence) store.getState().editEvidence(evidence.id);
  }
  document.body.dataset.demoReady = "1";
}

void main();
