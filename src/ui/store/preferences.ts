export interface Preferences {
  getLastProject(): string | null;
  setLastProject(id: string | null): void;
}

const KEY = "qapture.lastProject";

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
  };
}

export function memoryPreferences(initial: string | null = null): Preferences {
  let last = initial;
  return {
    getLastProject: () => last,
    setLastProject: (id) => {
      last = id;
    },
  };
}
