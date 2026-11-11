import type { StoreApi } from "zustand/vanilla";
import { fixtureProblem, type Fixture } from "../../domain/fixtures";
import type { Repositories } from "../../ports/repositories";
import type { SliceHelpers } from "./evidence-slice";
import type { WorkspaceState } from "./workspace";

export interface FixtureSlice {
  fixtures: Fixture[];
  fixturesCaseId: string | null;
  loadFixtures(caseId: string): Promise<void>;
  addFixtures(caseId: string, files: File[]): Promise<void>;
  removeFixture(id: string): Promise<void>;
  revealFixture(id: string): Promise<void>;
  copyFixturePath(id: string): Promise<void>;
}

type Set = StoreApi<WorkspaceState>["setState"];
type Get = StoreApi<WorkspaceState>["getState"];

export function createFixtureSlice(
  repos: Repositories,
  set: Set,
  get: Get,
  h: SliceHelpers,
): FixtureSlice {
  return {
    fixtures: [],
    fixturesCaseId: null,

    async loadFixtures(caseId: string): Promise<void> {
      // Al cambiar de caso se vacía la lista: nunca se muestran (ni se quitan) archivos de otro caso.
      if (get().fixturesCaseId !== caseId) set({ fixturesCaseId: caseId, fixtures: [] });
      try {
        const fixtures = await repos.fixtures.list(caseId);
        if (get().fixturesCaseId !== caseId) return;
        set({ fixtures });
      } catch (error) {
        h.report(error);
      }
    },

    async addFixtures(caseId: string, files: File[]): Promise<void> {
      for (const file of files) {
        const problem = fixtureProblem(file.name, file.size);
        if (problem !== null) {
          get().pushToast("error", problem);
          continue;
        }
        try {
          const buffer = await file.arrayBuffer();
          const bytes = new Uint8Array(buffer);
          await repos.fixtures.add(caseId, file.name, bytes);
        } catch (err) {
          h.report(err);
        }
      }
      await get().loadFixtures(caseId);
    },

    async removeFixture(id: string): Promise<void> {
      try {
        await repos.fixtures.remove(id);
        const caseId = get().fixturesCaseId;
        if (caseId) await get().loadFixtures(caseId);
      } catch (e) {
        h.report(e);
      }
    },

    async revealFixture(id: string): Promise<void> {
      try {
        await repos.fixtures.reveal(id);
      } catch (e) {
        h.report(e);
      }
    },

    async copyFixturePath(id: string): Promise<void> {
      try {
        await repos.fixtures.copyPath(id);
        get().pushToast("success", "Ruta copiada.");
      } catch (e) {
        h.report(e);
      }
    },
  };
}
