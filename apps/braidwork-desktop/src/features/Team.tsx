import { useI18n } from "../i18n/context";
import { Plus, ArrowUpRight } from "lucide-react";
import { useWorkspace } from "../app/context";
import { Badge, Empty, PageHeading } from "../components/Common";
export function Team() {
  const { t } = useI18n();
  const { snapshot, select, showForm } = useWorkspace();
  if (!snapshot) return null;
  return (
    <>
      <PageHeading
        title={t("Team")}
        subtitle={t(
          "Agents define a specialty. Sessions put that specialty to work.",
        )}
        action={
          <div className="actions">
            <button onClick={() => showForm({ kind: "agent" })}>
              <Plus size={16} /> {t("Create agent")}{" "}
            </button>
            <button
              className="primary"
              onClick={() => showForm({ kind: "session" })}
            >
              <Plus size={16} /> {t("Create session")}{" "}
            </button>
          </div>
        }
      />
      <section className="section">
        <div className="section-heading">
          <h2>{t("Sessions")}</h2>
          <span>
            {snapshot.sessions.length} {t("execution surfaces")}
          </span>
        </div>
        {snapshot.sessions.length ? (
          <div className="team-grid">
            {snapshot.sessions.map((session) => {
              const agent = snapshot.agents.find(
                (agent) =>
                  agent.id === session.agent_spec_id &&
                  agent.revision === session.agent_spec_revision,
              );
              const resource = snapshot.resources.find(
                (resource) => resource.id === session.resource_id,
              );
              return (
                <button
                  className="session-card"
                  key={session.id}
                  onClick={() => select({ kind: "session", id: session.id })}
                >
                  <div className="card-heading">
                    <strong>{agent?.name ?? t("Missing agent")}</strong>
                    <ArrowUpRight size={16} />
                  </div>
                  <p>{agent?.role ?? t("Unknown role")}</p>
                  <dl>
                    <dt>{t("Session")}</dt>
                    <dd>{session.label}</dd>
                    <dt>{t("Resource")}</dt>
                    <dd>{resource?.name ?? t("Missing resource")}</dd>
                    <dt>{t("Agent revision")}</dt>
                    <dd>
                      {agent?.name ?? t("Missing agent")} ·{" "}
                      {session.agent_spec_revision}
                    </dd>
                  </dl>
                  <Badge state={session.status} />
                  {session.external_ref && (
                    <small className="ellipsis">{session.external_ref}</small>
                  )}
                </button>
              );
            })}
          </div>
        ) : (
          <Empty
            title={t("No sessions yet.")}
            action={
              <button onClick={() => showForm({ kind: "session" })}>
                {" "}
                {t("Create session")}{" "}
              </button>
            }
          >
            {" "}
            {t(
              "A resource can support several independent, specialized conversations.",
            )}{" "}
          </Empty>
        )}
      </section>
      <section className="section">
        <div className="section-heading">
          <h2>{t("Agent definitions")}</h2>
          <span>{t("Exact revisions preserved")}</span>
        </div>
        {snapshot.agents.length ? (
          <div className="row-list">
            {snapshot.agents.map((agent) => (
              <button
                key={`${agent.id}@${agent.revision}`}
                className="entity-row"
                onClick={() =>
                  select({
                    kind: "agent",
                    id: agent.id,
                    revision: agent.revision,
                  })
                }
              >
                <span>
                  <strong>{agent.name}</strong>
                  <small>
                    {agent.role} {t("· revision")} {agent.revision}
                  </small>
                </span>
                <span className="muted">
                  {agent.delegation.allowed
                    ? t("Can delegate")
                    : t("Specialist")}
                </span>
              </button>
            ))}
          </div>
        ) : (
          <Empty
            title={t("No agents yet.")}
            action={
              <button onClick={() => showForm({ kind: "agent" })}>
                {" "}
                {t("Create agent")}{" "}
              </button>
            }
          >
            {" "}
            {t(
              "Define a role and mission for the work you want to delegate.",
            )}{" "}
          </Empty>
        )}
      </section>
    </>
  );
}
