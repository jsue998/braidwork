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
      <main className="start-screen">
        <h1>Something went wrong</h1>
        <p>Close and reopen Braidwork. Your project remains on disk.</p>
        {import.meta.env.DEV && (
          <details>
            <summary>Technical details</summary>
            <pre>{this.state.error.message}</pre>
          </details>
        )}
      </main>
    ) : (
      this.props.children
    );
  }
}
