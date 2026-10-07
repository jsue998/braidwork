import { useId, type ReactNode } from "react";
import type { IpcError } from "../types/ipc";
import { stateLabel } from "../lib/derive";
export function Badge({ state }: { state: string }) {
  return <span className={`badge state-${state}`}>{stateLabel(state)}</span>;
}
export function Empty({
  title,
  children,
  action,
}: {
  title: string;
  children?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="empty">
      <h3>{title}</h3>
      <p>{children}</p>
      {action}
    </div>
  );
}
export function ErrorNotice({
  error,
  dismiss,
}: {
  error: IpcError;
  dismiss?: () => void;
}) {
  return (
    <div role="alert" className="error-notice">
      <div>
        <strong>{error.message}</strong>
        <details>
          <summary>Technical details</summary>
          <pre>
            {error.code}
            {error.detail ? `\n${error.detail}` : ""}
          </pre>
        </details>
      </div>
      {dismiss && <button onClick={dismiss}>Dismiss</button>}
    </div>
  );
}
export function Field({
  label,
  children,
  hint,
}: {
  label: string;
  children: (id: string) => ReactNode;
  hint?: string;
}) {
  const id = useId();
  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      {children(id)}
      {hint && <small>{hint}</small>}
    </div>
  );
}
export function Technical({ children }: { children: ReactNode }) {
  return (
    <details className="technical">
      <summary>Technical details</summary>
      {children}
    </details>
  );
}
export function PageHeading({
  title,
  subtitle,
  action,
}: {
  title: string;
  subtitle: string;
  action?: ReactNode;
}) {
  return (
    <header className="page-heading">
      <div>
        <p className="eyebrow">PROJECT WORKSPACE</p>
        <h1>{title}</h1>
        <p>{subtitle}</p>
      </div>
      {action}
    </header>
  );
}

// Returned model text stays inert: React escapes it, with no Markdown/HTML parser.
export function ContentPreview({
  text,
  instructions = false,
}: {
  text: string;
  instructions?: boolean;
}) {
  return (
    <pre
      className={`content-preview${instructions ? " instructions-preview" : ""}`}
    >
      {text}
    </pre>
  );
}
