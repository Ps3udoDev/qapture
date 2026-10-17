import { createStore, type StoreApi } from "zustand/vanilla";
import * as useCases from "../../application/use-cases";
import type { CaseInput, CaseRecord, FormErrors } from "../../domain/cases";
import type { FieldInput, NewProjectInput, Project, ProjectMeta, ProjectSummary, Template } from "../../domain/project";
import type { Status } from "../../domain/status";
import { AppError } from "../../ports/errors";
import type { Repositories } from "../../ports/repositories";
import { createEvidenceSlice, type EvidenceSlice } from "./evidence-slice";
import { createDataSlice, type DataSlice } from "./data-slice";
import { createExportSlice, type ExportSlice } from "./export-slice";
import { createSettingsSlice, type SettingsSlice } from "./settings-slice";
import { browserPreferences, type Preferences } from "./preferences";

export type FormResult = { ok: true } | { ok: false; errors: FormErrors };
export interface ToastAction {
  label: string;
  run(): void;
}
export interface Toast {
  id: number;
  kind: "success" | "error";
  message: string;
  actions?: ToastAction[];
}

export interface WorkspaceState extends EvidenceSlice, ExportSlice, DataSlice, SettingsSlice {
  templates: Template[];
  projects: ProjectSummary[];
  project: Project | null;
  cases: CaseRecord[];
  selectedCaseId: string | null;
  search: string;
  statusFilter: Status | "";
  ready: boolean;
  toasts: Toast[];
  init(): Promise<void>;
  selectProject(id: string): Promise<void>;
  createProject(input: NewProjectInput): Promise<FormResult>;
  updateProjectMeta(meta: ProjectMeta): Promise<FormResult>;
  saveFields(fields: FieldInput[]): Promise<FormResult>;
  deleteProject(): Promise<void>;
  saveTemplateFromProject(name: string, testType: string): Promise<FormResult>;
  deleteTemplate(id: string): Promise<void>;
  saveCase(input: CaseInput, caseId?: string): Promise<FormResult>;
  duplicateCase(id: string): Promise<void>;
  deleteCase(id: string): Promise<void>;
  selectCase(id: string | null): void;
  setSearch(search: string): void;
  setStatusFilter(status: Status | ""): void;
  pushToast(kind: Toast["kind"], message: string, actions?: ToastAction[]): void;
  dismissToast(id: number): void;
}

const NO_PROJECT: FormResult = { ok: false, errors: { _form: "No hay un proyecto abierto." } };

export function createWorkspaceStore(repos: Repositories, prefs: Preferences = browserPreferences()): StoreApi<WorkspaceState> {
  let toastSeq = 0;

  const store = createStore<WorkspaceState>()((set, get) => {
    const report = (error: unknown) => {
      const message = error instanceof AppError ? error.message : "Ocurrió un error inesperado.";
      get().pushToast("error", message);
    };

    const refreshLists = async () => {
      const [templates, projects] = await Promise.all([repos.templates.list(), repos.projects.list()]);
      set({ templates, projects });
    };

    const loadProject = async (id: string | null, keepSelection = false) => {
      if (!id) {
        set({ project: null, cases: [], selectedCaseId: null });
        prefs.setLastProject(null);
        return;
      }
      const [project, cases] = await Promise.all([repos.projects.get(id), repos.cases.list(id)]);
      const previous = get().selectedCaseId;
      const selectedCaseId = keepSelection && cases.some((c) => c.id === previous) ? previous : (cases[0]?.id ?? null);
      set(keepSelection ? { project, cases, selectedCaseId } : { project, cases, selectedCaseId, search: "", statusFilter: "" });
      prefs.setLastProject(id);
    };

    /** Ejecuta una acción de formulario: errores de validación → formulario; el resto → toast. */
    const form = async <T>(run: () => Promise<useCases.Outcome<T>>, onOk: (value: T) => Promise<void> | void): Promise<FormResult> => {
      try {
        const outcome = await run();
        if (!outcome.ok) return outcome;
        await onOk(outcome.value);
        return { ok: true };
      } catch (error) {
        report(error);
        return { ok: false, errors: {} };
      }
    };

    const refreshCases = async () => {
      const current = get().project;
      if (current) set({ cases: await repos.cases.list(current.id) });
    };
    const reload = async (projectId: string | null = null) => {
      await refreshLists();
      const { projects, project } = get();
      const exists = (id: string | null | undefined): id is string => !!id && projects.some((p) => p.id === id);
      const currentId = project?.id;
      const target = exists(projectId) ? projectId : exists(currentId) ? currentId : (projects[0]?.id ?? null);
      await loadProject(target);
      await get().loadEvidence();
    };
    /** Una sola vez: el tester que F4 guardaba en el equipo pasa al perfil si este está vacío. */
    const migrateTester = async () => {
      const snapshot = get().settings;
      const legacy = prefs.getTester().trim();
      if (!snapshot || snapshot.settings.profile.tester || !legacy) return;
      // Silencioso; `persistSettings` ya avisa los errores que no son de validación.
      await get().persistSettings({ ...snapshot.settings, profile: { ...snapshot.settings.profile, tester: legacy } });
    };
    const helpers = { report, refreshCases, form, reload };

    return {
      templates: [],
      projects: [],
      project: null,
      cases: [],
      selectedCaseId: null,
      search: "",
      statusFilter: "",
      ready: false,
      toasts: [],

      async init() {
        try {
          const status = await repos.data.startupStatus();
          if (status.recovery) {
            // Base dañada: no se cargan proyectos; la pantalla de recuperación ofrece los respaldos.
            set({ recovery: status });
            // El comando de ajustes funciona en recuperación; la migración del tester espera a una base sana.
            await get().loadSettings();
            await get().loadBackups();
            return;
          }
          await get().loadSettings();
          await migrateTester();
          await refreshLists();
          const { projects } = get();
          const last = prefs.getLastProject();
          await loadProject(projects.find((p) => p.id === last)?.id ?? projects[0]?.id ?? null);
        } catch (error) {
          report(error);
        } finally {
          set({ ready: true });
        }
      },

      async selectProject(id) {
        try {
          await loadProject(id);
        } catch (error) {
          report(error);
        }
      },

      createProject: (input) =>
        form(
          () => useCases.createProject(repos, input),
          async (project) => {
            await refreshLists();
            await loadProject(project.id);
            get().pushToast("success", `Matriz «${project.name}» creada.`);
          },
        ),

      async updateProjectMeta(meta) {
        const current = get().project;
        if (!current) return NO_PROJECT;
        return form(
          () => useCases.updateProjectMeta(repos, current.id, meta),
          async (project) => {
            set({ project });
            await refreshLists();
            get().pushToast("success", "Datos del proyecto guardados.");
          },
        );
      },

      async saveFields(fields) {
        const current = get().project;
        if (!current) return NO_PROJECT;
        return form(
          () => useCases.saveProjectFields(repos, current.id, fields),
          async () => {
            await loadProject(current.id, true);
            get().pushToast("success", "Columnas guardadas.");
          },
        );
      },

      async deleteProject() {
        const current = get().project;
        if (!current) return;
        try {
          await repos.projects.remove(current.id);
          await refreshLists();
          await loadProject(get().projects[0]?.id ?? null);
          get().pushToast("success", `Matriz «${current.name}» eliminada.`);
        } catch (error) {
          report(error);
        }
      },

      async saveTemplateFromProject(name, testType) {
        const current = get().project;
        if (!current) return NO_PROJECT;
        return form(
          () => useCases.saveTemplate(repos, current.id, name, testType),
          async (template) => {
            await refreshLists();
            get().pushToast("success", `Plantilla «${template.name}» guardada.`);
          },
        );
      },

      async deleteTemplate(id) {
        try {
          await repos.templates.remove(id);
          await refreshLists();
          get().pushToast("success", "Plantilla eliminada.");
        } catch (error) {
          report(error);
        }
      },

      async saveCase(input, caseId) {
        const current = get().project;
        if (!current) return NO_PROJECT;
        const result = await form(
          () => useCases.saveCase(repos, current, input, caseId),
          async (saved) => {
            set((s) => ({
              cases: caseId ? s.cases.map((c) => (c.id === saved.id ? saved : c)) : [...s.cases, saved],
              selectedCaseId: saved.id,
            }));
            await refreshLists();
            get().pushToast("success", caseId ? `${saved.code} actualizado.` : `${saved.code} creado.`);
          },
        );
        if (!result.ok && result.errors._form) {
          // Las columnas pudieron cambiar mientras el formulario estaba abierto: recargarlas
          // para que el siguiente intento se normalice con las columnas vigentes.
          try {
            set({ project: await repos.projects.get(current.id) });
          } catch (error) {
            report(error);
          }
        }
        return result;
      },

      async duplicateCase(id) {
        try {
          const source = get().cases.find((c) => c.id === id);
          const copy = await repos.cases.duplicate(id);
          set((s) => ({ cases: [...s.cases, copy], selectedCaseId: copy.id }));
          await refreshLists();
          get().pushToast("success", source ? `${copy.code} creado a partir de ${source.code}.` : `${copy.code} creado.`);
        } catch (error) {
          report(error);
        }
      },

      async deleteCase(id) {
        try {
          const { cases } = get();
          const index = cases.findIndex((c) => c.id === id);
          const removed = cases[index];
          await repos.cases.remove(id);
          const rest = cases.filter((c) => c.id !== id);
          set({ cases: rest, selectedCaseId: rest[Math.max(0, index - 1)]?.id ?? null });
          await refreshLists();
          if (removed) get().pushToast("success", `${removed.code} eliminado.`);
        } catch (error) {
          report(error);
        }
      },

      selectCase: (id) => set({ selectedCaseId: id }),
      setSearch: (search) => set({ search }),
      setStatusFilter: (statusFilter) => set({ statusFilter }),
      pushToast: (kind, message, actions) => set((s) => ({ toasts: [...s.toasts, { id: ++toastSeq, kind, message, actions }] })),
      dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
      ...createEvidenceSlice(repos, set, get, helpers),
      ...createExportSlice(repos, set, get, helpers),
      ...createDataSlice(repos, set, get, helpers),
      ...createSettingsSlice(repos, set, get, helpers),
    };
  });

  // Las evidencias siguen al caso seleccionado, sin importar qué acción cambió la selección.
  store.subscribe((state, previous) => {
    if (state.selectedCaseId !== previous.selectedCaseId) void state.loadEvidence();
    // Y las versiones del documento siguen al proyecto abierto.
    if (state.project?.id !== previous.project?.id) void state.loadDocVersions();
  });
  return store;
}
