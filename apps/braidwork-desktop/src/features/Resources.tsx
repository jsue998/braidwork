import { useI18n } from "../i18n/context";
import { Plus } from "lucide-react";
import { useWorkspace } from "../app/context";
import { sessionsByResource, stateLabel } from "../lib/derive";
import { Badge, Empty, PageHeading } from "../components/Common";
export function Resources() {
  const { t } = useI18n();
  const { snapshot, select, showForm } = useWorkspace();
  if (!snapshot) return null;
  return (
    <>
      <PageHeading
        title={t("AI resources")}
        subtitle={t(
          "The AI access you already have. No credentials or provider connections required.",
        )}
        action={
          <button
            className="primary"
            onClick={() => showForm({ kind: "resource" })}
          >
            <Plus size={16} /> {t("Create resource")}{" "}
          </button>
        }
      />
      {snapshot.resources.length ? (
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>{t("Name")}</th>
                <th>{t("Access")}</th>
                <th>{t("Scarcity")}</th>
                <th>{t("Status")}</th>
                <th>{t("Sessions")}</th>
              </tr>
            </thead>
            <tbody>
              {sessionsByResource(snapshot).map(({ resource, sessions }) => (
                <tr key={resource.id}>
                  <td>
                    <button
                      className="text-button"
                      onClick={() =>
                        select({ kind: "resource", id: resource.id })
                      }
                    >
                      {resource.name}
                    </button>
                    <small>{resource.provider}</small>
                  </td>
                  <td>{stateLabel(resource.access_mode, t)}</td>
                  <td>{stateLabel(resource.scarcity, t)}</td>
                  <td>
                    <Badge state={resource.status} />
                  </td>
                  <td>{sessions.length}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <Empty
          title={t("No AI resources yet.")}
          action={
            <button onClick={() => showForm({ kind: "resource" })}>
              {" "}
              {t("Create resource")}{" "}
            </button>
          }
        >
          {" "}
          {t(
            "Describe an account, API, harness, or local model access. Quotas are not inferred.",
          )}{" "}
        </Empty>
      )}
    </>
  );
}
