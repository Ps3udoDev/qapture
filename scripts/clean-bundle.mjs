// Borra los instaladores de builds anteriores antes de `pnpm tauri build` (beforeBuildCommand),
// para que src-tauri/target/release/bundle solo tenga los de la versión que se está generando.
// Solo borra archivos: las carpetas (msi/, nsis/…) se reutilizan en el build siguiente.
import { readdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const bundle = join(dirname(fileURLToPath(import.meta.url)), "..", "src-tauri", "target", "release", "bundle");

function clean(dir) {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return 0; // Primer build: todavía no hay carpeta bundle.
  }
  let removed = 0;
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      removed += clean(path);
    } else {
      // Reintentos: el antivirus puede tener abierto un .exe recién generado unos instantes.
      rmSync(path, { force: true, maxRetries: 5, retryDelay: 300 });
      removed += 1;
    }
  }
  return removed;
}

console.log(`Instaladores anteriores eliminados: ${clean(bundle)}`);
