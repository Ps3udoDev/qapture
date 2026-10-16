export interface Preferences {
  getLastProject(): string | null;
  setLastProject(id: string | null): void;
  /** Nombre del tester para la carátula. F6 lo reemplaza por el perfil de Settings. */
  getTester(): string;
  setTester(name: string): void;
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
    setTester(name) {
      try {
        localStorage.setItem(TESTER_KEY, name);
      } catch {
        // Igual que arriba: sin almacenamiento solo se pierde la sugerencia.
      }
    },
  };
}

export function memoryPreferences(initial: string | null = null, initialTester = ""): Preferences {
  let last = initial;
  let tester = initialTester;
  return {
    getLastProject: () => last,
    setLastProject: (id) => {
      last = id;
    },
    getTester: () => tester,
    setTester: (name) => {
      tester = name;
    },
  };
}
