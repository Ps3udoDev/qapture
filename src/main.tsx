import "./ui/theme/tokens.css";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { tauriRepositories } from "./infrastructure/tauri/repositories";
import { createWorkspaceStore } from "./ui/store/workspace";

const store = createWorkspaceStore(tauriRepositories());

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App store={store} />
  </React.StrictMode>,
);
