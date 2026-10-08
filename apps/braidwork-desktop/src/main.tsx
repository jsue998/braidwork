import { PreferencesProvider } from "./i18n/PreferencesProvider";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { WorkspaceProvider } from "./app/WorkspaceProvider";
import "./styles/app.css";
const root = document.getElementById("root");
if (!root) throw new Error("Application root is missing");
createRoot(root).render(
  <StrictMode>
    <PreferencesProvider>
      <ErrorBoundary>
        <WorkspaceProvider>
          <App />
        </WorkspaceProvider>
      </ErrorBoundary>
    </PreferencesProvider>
  </StrictMode>,
);
