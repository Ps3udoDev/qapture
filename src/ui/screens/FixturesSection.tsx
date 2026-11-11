import { Copy, FolderOpen, Plus, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { formatSize } from "../../domain/data";
import { FIXTURE_ACCEPT, type Fixture } from "../../domain/fixtures";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { btn } from "../components/styles";
import { useWorkspace } from "../store/context";

export function FixturesSection({ caseId }: { caseId: string }) {
  const fixtures = useWorkspace((s) => s.fixtures);
  const loadFixtures = useWorkspace((s) => s.loadFixtures);
  const addFixtures = useWorkspace((s) => s.addFixtures);
  const removeFixture = useWorkspace((s) => s.removeFixture);
  const revealFixture = useWorkspace((s) => s.revealFixture);
  const copyFixturePath = useWorkspace((s) => s.copyFixturePath);

  const fileInputRef = useRef<HTMLInputElement>(null);
  const [deleting, setDeleting] = useState<Fixture | null>(null);
  const [dropping, setDropping] = useState(false);

  useEffect(() => {
    void loadFixtures(caseId);
  }, [caseId, loadFixtures]);

  return (
    <section
      aria-label="Archivos de prueba"
      className={`mt-6 rounded-xl transition-colors ${dropping ? "bg-t4s-blue/5 outline-2 outline-dashed outline-t4s-blue" : ""}`}
      onDragOver={(e) => {
        e.preventDefault();
        e.stopPropagation();
        setDropping(true);
      }}
      onDragLeave={() => setDropping(false)}
      onDrop={async (e) => {
        e.preventDefault();
        e.stopPropagation();
        setDropping(false);
        const files = Array.from(e.dataTransfer.files ?? []);
        if (files.length > 0) {
          await addFixtures(caseId, files);
        }
      }}
    >
      <div className="flex items-center justify-between">
        <h3 className="font-semibold">Archivos de prueba</h3>
        <button type="button" className={btn.secondary} onClick={() => fileInputRef.current?.click()}>
          <Plus size={16} /> Agregar…
        </button>
        <input
          ref={fileInputRef}
          type="file"
          multiple
          accept={FIXTURE_ACCEPT}
          className="hidden"
          tabIndex={-1}
          aria-label="Agregar archivos de prueba"
          onChange={async (e) => {
            const files = Array.from(e.target.files ?? []);
            e.target.value = "";
            if (files.length > 0) {
              await addFixtures(caseId, files);
            }
          }}
        />
      </div>

      {fixtures.length === 0 ? (
        <p className="mt-3 rounded-xl border border-dashed border-t4s-line p-6 text-center text-sm text-t4s-gray">
          Sin archivos de prueba. Arrastra aquí o usa «Agregar…».
        </p>
      ) : (
        <div className="mt-3 overflow-hidden rounded-xl border border-t4s-line bg-white">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-t4s-line bg-t4s-cream/40 text-left text-xs uppercase tracking-wide text-t4s-gray">
                <th className="px-4 py-2 font-medium">Nombre</th>
                <th className="px-4 py-2 font-medium">Tamaño</th>
                <th className="px-4 py-2 font-medium">Nota</th>
                <th className="px-4 py-2 text-right font-medium">
                  <span className="sr-only">Acciones</span>
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-t4s-line">
              {fixtures.map((fixture) => (
                <tr key={fixture.id} className="hover:bg-t4s-cream/20">
                  <td className="px-4 py-2.5 font-medium text-t4s-charcoal">{fixture.fileName}</td>
                  <td className="px-4 py-2.5 text-t4s-gray">{formatSize(fixture.size)}</td>
                  <td className="px-4 py-2.5 text-t4s-gray">{fixture.note.trim() !== "" ? fixture.note : null}</td>
                  <td className="px-4 py-2.5 text-right">
                    <div className="flex items-center justify-end gap-1">
                      <button type="button" className={btn.ghost} onClick={() => void revealFixture(fixture.id)}>
                        <FolderOpen size={14} /> Mostrar en carpeta
                      </button>
                      <button type="button" className={btn.ghost} onClick={() => void copyFixturePath(fixture.id)}>
                        <Copy size={14} /> Copiar ruta
                      </button>
                      <button type="button" className={btn.ghost} onClick={() => setDeleting(fixture)}>
                        <Trash2 size={14} /> Quitar
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {deleting && (
        <ConfirmDialog
          title="Quitar archivo de prueba"
          message={`Se quitará «${deleting.fileName}» de este caso.`}
          confirmLabel="Quitar"
          onCancel={() => setDeleting(null)}
          onConfirm={async () => {
            const targetId = deleting.id;
            setDeleting(null);
            await removeFixture(targetId);
          }}
        />
      )}
    </section>
  );
}
