import { z } from "zod";
import type { Fixture } from "../../domain/fixtures";
import type { FixtureStore } from "../../ports/repositories";
import { call } from "./client";
import { fixtureSchema } from "./dto";
import { encodeFrame } from "./frame";

const nothing = z.null();

export function createTauriFixtures(): FixtureStore {
  return {
    list: (caseId: string): Promise<Fixture[]> =>
      call("fixture_list", { caseId }, z.array(fixtureSchema)),

    add: (caseId: string, fileName: string, bytes: Uint8Array): Promise<Fixture> =>
      call("fixture_add", encodeFrame({ caseId, fileName, note: "" }, bytes), fixtureSchema),

    remove: async (id: string): Promise<void> => {
      await call("fixture_remove", { id }, nothing);
    },

    reveal: async (id: string): Promise<void> => {
      await call("fixture_reveal", { id }, nothing);
    },

    copyPath: async (id: string): Promise<void> => {
      await call("fixture_copy_path", { id }, nothing);
    },
  };
}

export const tauriFixtures = createTauriFixtures;
