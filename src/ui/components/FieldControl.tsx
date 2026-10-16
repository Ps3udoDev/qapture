import type { FieldDef } from "../../domain/fields";
import { inputCls } from "./styles";

export function FieldControl({ id, field, value, onChange }: {
  id: string;
  field: FieldDef;
  value: string;
  onChange: (value: string) => void;
}) {
  const common = { id, className: inputCls, value };
  switch (field.type) {
    case "longtext":
      return <textarea {...common} rows={4} onChange={(e) => onChange(e.target.value)} />;
    case "list":
      return (
        <select {...common} onChange={(e) => onChange(e.target.value)}>
          <option value="">—</option>
          {field.options.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>
      );
    case "boolean":
      return (
        <select {...common} onChange={(e) => onChange(e.target.value)}>
          <option value="">—</option>
          <option value="true">Sí</option>
          <option value="false">No</option>
        </select>
      );
    case "date":
      return <input {...common} type="date" onChange={(e) => onChange(e.target.value)} />;
    case "number":
      return <input {...common} inputMode="decimal" onChange={(e) => onChange(e.target.value)} />;
    default:
      return <input {...common} type="text" onChange={(e) => onChange(e.target.value)} />;
  }
}

export function displayValue(field: FieldDef, value: string): string {
  if (!value) return "—";
  if (field.type === "boolean") return value === "true" ? "Sí" : "No";
  if (field.type === "date") {
    const [y, m, d] = value.split("-");
    return `${d}/${m}/${y}`;
  }
  return value;
}
