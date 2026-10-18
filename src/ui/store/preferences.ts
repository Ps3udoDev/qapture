export interface Preferences {
  getLastProject(): string | null;
  setLastProject(id: string | null): void;
  /** Solo para migrar el tester de F4 al perfil de Ajustes; ya no se escribe. */
  getTester(): string;
}

const KEY = "qapture.lastProject";
const TESTER_KEY = "qapture.tester";

export function browserPreferences(): Preferences {
  return {
    getLastProject() {
      try {
        return localStorage.getItem(KEY);
      } catch {
        return null;
      }
    },
    setLastProject(id) {
      try {
        if (id) localStorage.setItem(KEY, id);
        else localStorage.removeItem(KEY);
      } catch {
        // Sin almacenamiento local: la app sigue funcionando, solo no recuerda el proyecto.
      }
    },
    getTester() {
      try {
        return localStorage.getItem(TESTER_KEY) ?? "";
      } catch {
        return "";
      }
    },
  };
}

export function memoryPreferences(initial: string | null = null, initialTester = ""): Preferences {
  let last = initial;
  return {
    getLastProject: () => last,
    setLastProject: (id) => {
      last = id;
    },
    getTester: () => initialTester,
  };
}
