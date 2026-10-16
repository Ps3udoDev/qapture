import { Trash2 } from "lucide-react";
import { useState, type FormEvent } from "react";
import type { FormErrors } from "../../domain/cases";
import { FormField } from "../components/FormField";
import { btn, iconButton, inputCls } from "../components/styles";
import { useWorkspace } from "../store/context";

export function TemplatesPanel() {
  const templates = useWorkspace((s) => s.templates);
  const project = useWorkspace((s) => s.project);
  const saveTemplate = useWorkspace((s) => s.saveTemplateFromProject);
  const deleteTemplate = useWorkspace((s) => s.deleteTemplate);
  const [name, setName] = useState("");
  const [testType, setTestType] = useState(project?.testType ?? "");
  const [errors, setErrors] = useState<FormErrors>({});

  async function submit(e: FormEvent) {
    e.preventDefault();
    const result = await saveTemplate(name, testType);
    if (result.ok) {
      setName("");
      setErrors({});
    } else setErrors(result.errors);
  }

  return (
    <div className="space-y-6">
      <form onSubmit={submit} noValidate className="grid grid-cols-2 gap-4">
        <p className="col-span-2 text-sm text-t4s-gray">Guarda las columnas activas de este proyecto para reutilizarlas al crear nuevas matrices.</p>
        <FormField label="Nombre de la plantilla" required htmlFor="tpl-name" error={errors.name ?? errors._form}>
          <input id="tpl-name" className={inputCls} value={name} onChange={(e) => setName(e.target.value)} placeholder="Ej. ACME Cobranzas" />
        </FormField>
        <FormField label="Tipo de prueba" htmlFor="tpl-type">
          <input id="tpl-type" className={inputCls} value={testType} onChange={(e) => setTestType(e.target.value)} />
        </FormField>
        <div className="col-span-2 flex justify-end">
          <button type="submit" className={btn.primary}>
            Guardar como plantilla
          </button>
        </div>
      </form>
      <section>
        <h3 className="mb-2 text-sm font-medium">Plantillas disponibles</h3>
        <ul className="divide-y divide-t4s-line rounded-lg border border-t4s-line">
          {templates.map((t) => (
            <li key={t.id} className="flex items-center justify-between px-3 py-2 text-sm">
              <span>
                <span className="font-medium">{t.name}</span>
                <span className="ml-2 text-xs text-t4s-gray">
                  {t.fields.length} columnas{t.builtin ? " · incluida" : ""}
                </span>
              </span>
              {!t.builtin && (
                <button type="button" aria-label={`Eliminar plantilla ${t.name}`} className={iconButton} onClick={() => void deleteTemplate(t.id)}>
                  <Trash2 size={16} />
                </button>
              )}
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
