import { useI18n } from "../i18n/context";
import { useId, type ReactNode } from "react";
import { errorMessage } from "../i18n";
import type { IpcError } from "../types/ipc";
import { stateLabel } from "../lib/derive";
export function Badge({ state }: { state: string }) {
  const { t } = useI18n();
  return <span className={`badge state-${state}`}>{stateLabel(state, t)}</span>;
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
  const { t } = useI18n();
  return (
    <div role="alert" className="error-notice">
      <div>
        <strong>{errorMessage(error, t)}</strong>
        <details>
          <summary>{t("Technical details")}</summary>
          <pre>
            {error.code}
            {`\n${error.message}`}
            {error.detail ? `\n${error.detail}` : ""}
          </pre>
        </details>
      </div>
      {dismiss && <button onClick={dismiss}>{t("Dismiss")}</button>}
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
  const { t } = useI18n();
  return (
    <details className="technical">
      <summary>{t("Technical details")}</summary>
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
  const { t } = useI18n();
  return (
    <header className="page-heading">
      <div>
        <p className="eyebrow">{t("PROJECT WORKSPACE")}</p>
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
