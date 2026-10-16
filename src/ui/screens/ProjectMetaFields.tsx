import type { ChangeEvent } from "react";
import type { FormErrors } from "../../domain/cases";
import type { ProjectMeta } from "../../domain/project";
import { FormField } from "../components/FormField";
import { inputCls } from "../components/styles";

export function ProjectMetaFields({ idPrefix, meta, errors, onChange }: {
  idPrefix: string;
  meta: ProjectMeta;
  errors: FormErrors;
  onChange: (meta: ProjectMeta) => void;
}) {
  const bind = (key: keyof ProjectMeta) => ({
    id: `${idPrefix}-${key}`,
    value: meta[key],
    className: inputCls,
    onChange: (e: ChangeEvent<HTMLInputElement>) => onChange({ ...meta, [key]: e.target.value }),
  });
  return (
    <>
      <FormField label="Nombre del proyecto o módulo" required htmlFor={`${idPrefix}-name`} error={errors.name} className="col-span-2">
        <input {...bind("name")} placeholder="Ej. Aplicación de recibos" />
      </FormField>
      <FormField label="Cliente" htmlFor={`${idPrefix}-client`} error={errors.client}>
        <input {...bind("client")} placeholder="Ej. GMX" />
      </FormField>
      <FormField label="Tipo de prueba" htmlFor={`${idPrefix}-testType`} error={errors.testType}>
        <input {...bind("testType")} placeholder="Ej. Funcional" />
      </FormField>
      <FormField label="Versión / release" htmlFor={`${idPrefix}-releaseVersion`} error={errors.releaseVersion} className="col-span-2">
        <input {...bind("releaseVersion")} placeholder="Ej. 2.4.0" />
      </FormField>
      <FormField label="Fecha de inicio" htmlFor={`${idPrefix}-startDate`} error={errors.startDate}>
        <input type="date" {...bind("startDate")} />
      </FormField>
      <FormField label="Fecha de fin" htmlFor={`${idPrefix}-endDate`} error={errors.endDate}>
        <input type="date" {...bind("endDate")} />
      </FormField>
    </>
  );
}
