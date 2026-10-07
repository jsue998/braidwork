import { useWorkspace } from "./app/context";
import { AppShell } from "./components/AppShell";
import { StartScreen } from "./features/StartScreen";
import { Overview } from "./features/Overview";
import { Team } from "./features/Team";
import { Work } from "./features/Work";
import { Resources } from "./features/Resources";
import { Results } from "./features/Results";
import { CreationForms } from "./features/CreationForms";
export function App() {
  const { snapshot, view } = useWorkspace();
  if (!snapshot) return <StartScreen />;
  return (
    <>
      <AppShell>
        {view === "Overview" ? (
          <Overview />
        ) : view === "Team" ? (
          <Team />
        ) : view === "Work" ? (
          <Work />
        ) : view === "Resources" ? (
          <Resources />
        ) : (
          <Results />
        )}
      </AppShell>
      <CreationForms />
    </>
  );
}
