import { useI18n } from "../i18n/context";
import { Component, type ReactNode } from "react";
export class ErrorBoundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state: { error: Error | null } = { error: null };
  static getDerivedStateFromError(error: Error) {
    return { error };
  }
  render() {
    return this.state.error ? (
      <RenderFailure error={this.state.error} />
    ) : (
      this.props.children
    );
  }
}

function RenderFailure({ error }: { error: Error }) {
  const { t } = useI18n();
  return (
    <main className="start-screen">
      <div className="start-content">
        <h1>{t("Something went wrong")}</h1>
        <p>{t("Close and reopen Braidwork. Your project remains on disk.")}</p>
        {import.meta.env.DEV && (
          <details>
            <summary>{t("Technical details")}</summary>
            <pre>{error.message}</pre>
          </details>
        )}
      </div>
    </main>
  );
}
