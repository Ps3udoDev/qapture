import "./ui/theme/tokens.css";
import ReactDOM from "react-dom/client";
import { tauriOverlayApi } from "./infrastructure/tauri/capture";
import { OverlayApp } from "./ui/overlay/OverlayApp";

// Sin StrictMode: el doble montaje pediría dos veces la imagen del monitor.
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(<OverlayApp api={tauriOverlayApi()} />);
