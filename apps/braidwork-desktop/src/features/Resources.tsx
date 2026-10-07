import { Plus } from "lucide-react";
import { useWorkspace } from "../app/context";
import { sessionsByResource, stateLabel } from "../lib/derive";
import { Badge, Empty, PageHeading } from "../components/Common";
export function Resources() {
  const { snapshot, select, showForm } = useWorkspace();
  if (!snapshot) return null;
  return (
    <>
      <PageHeading
        title="AI resources"
        subtitle="The AI access you already have. No credentials or provider connections required."
        action={
          <button
            className="primary"
            onClick={() => showForm({ kind: "resource" })}
          >
            <Plus size={16} /> Create resource
          </button>
        }
      />
      {snapshot.resources.length ? (
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Access</th>
                <th>Scarcity</th>
                <th>Status</th>
                <th>Sessions</th>
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
                  <td>{stateLabel(resource.access_mode)}</td>
                  <td>{stateLabel(resource.scarcity)}</td>
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
          title="No AI resources yet."
          action={
            <button onClick={() => showForm({ kind: "resource" })}>
              Create resource
            </button>
          }
        >
          Describe an account, API, harness, or local model access. Quotas are
          not inferred.
        </Empty>
      )}
    </>
  );
}
