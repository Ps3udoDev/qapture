import { beforeEach, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import fixture from "./__fixtures__/contracts.json";
import { guionImportInputSchema } from "./dto";
import { tauriGuion } from "./guion";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const mockedInvoke = vi.mocked(invoke);
beforeEach(() => mockedInvoke.mockReset());
it("envía el token y la definición con los nombres de comandos del contrato", async () => {
  const guion = tauriGuion();
  const input = guionImportInputSchema.parse(fixture.inputs.guionImportInput);
  mockedInvoke.mockResolvedValueOnce(fixture.guionInspect).mockResolvedValueOnce(fixture.guionPreview).mockResolvedValueOnce(fixture.importSummary).mockResolvedValueOnce([fixture.importProfile]).mockResolvedValueOnce(null);
  await guion.inspect();
  await guion.preview(input.token, input.profile);
  await guion.import(input);
  await guion.listProfiles();
  await guion.deleteProfile("perfil-1");
  expect(mockedInvoke.mock.calls).toEqual([
    ["guion_inspect", {}], ["guion_preview", { token: input.token, profile: input.profile }],
    ["guion_import", { input }], ["import_profiles_list", {}], ["import_profile_delete", { id: "perfil-1" }],
  ]);
});
