import { Plus } from "lucide-react";
import logo from "../../assets/brand/t4s-logo.png";
import { btn } from "../components/styles";

export function EmptyState({ onCreate }: { onCreate: () => void }) {
  return (
    <main className="grid flex-1 place-items-center p-8">
      <div className="max-w-md text-center">
        <img src={logo} alt="Teams4Soft" className="mx-auto mb-6 h-28" />
        <h1 className="text-2xl font-semibold">Crea tu primera matriz</h1>
        <p className="mt-2 text-t4s-gray">
          Una matriz agrupa los casos de prueba de un proyecto o módulo. Elige una plantilla de columnas y empieza a registrar ejecuciones.
        </p>
        <button type="button" className={`${btn.primary} mt-6`} onClick={onCreate}>
          <Plus size={16} /> Nueva matriz
        </button>
      </div>
    </main>
  );
}
