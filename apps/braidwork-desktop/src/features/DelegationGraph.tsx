import { useI18n } from "../i18n/context";
import { useEffect, useMemo } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  Handle,
  Position,
  useNodesState,
  useEdgesState,
  type NodeProps,
  type Node,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { useWorkspace } from "../app/context";
import { delegationGraph } from "../lib/derive";
import { Empty } from "../components/Common";
type AssignmentNode = Node<
  { title: string; role: string; session: string; status: string },
  "assignment"
>;
function WorkNode({ data, selected }: NodeProps<AssignmentNode>) {
  return (
    <div className={`graph-node ${selected ? "selected" : ""}`}>
      <Handle type="target" position={Position.Left} />
      <strong>{data.title}</strong>
      <span>
        {data.role} · {data.session}
      </span>
      <small>{data.status}</small>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}
const nodeTypes = { assignment: WorkNode };
export function DelegationGraph() {
  const { t } = useI18n();
  const { snapshot, select } = useWorkspace();
  const graph = useMemo(
    () => (snapshot ? delegationGraph(snapshot, t) : { nodes: [], edges: [] }),
    [snapshot, t],
  );
  const [nodes, setNodes, onNodesChange] = useNodesState(graph.nodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(graph.edges);
  useEffect(() => {
    setNodes((previous) => {
      let next = previous.length;
      return graph.nodes.map((node) => {
        const existing = previous.find((old) => old.id === node.id);
        const position = existing?.position ?? {
          x: (next % 3) * 310,
          y: Math.floor(next / 3) * 180,
        };
        if (!existing) next += 1;
        return { ...node, position };
      });
    });
    setEdges(graph.edges);
  }, [graph, setNodes, setEdges]);
  if (!snapshot?.assignments.length)
    return (
      <Empty title={t("No assigned work to connect.")}>
        {" "}
        {t(
          "Create assignments, then record an explicit delegation between them.",
        )}{" "}
      </Empty>
    );
  return (
    <>
      <p className="muted">
        {" "}
        {t(
          "Nodes are assignments. Edges record who delegated work to whom. Task dependencies are separate.",
        )}{" "}
      </p>
      <div
        className="graph"
        role="region"
        aria-label={t("Recorded assignment delegations")}
      >
        <ReactFlow
          ariaLabelConfig={{
            "controls.ariaLabel": t("Graph controls"),
            "node.a11yDescription.default": t(
              "Enter or Space selects an assignment. Arrow keys move it; Escape clears selection.",
            ),
            "node.a11yDescription.keyboardDisabled": t(
              "Enter or Space selects an assignment. Arrow keys move it; Escape clears selection.",
            ),
            "edge.a11yDescription.default": t(
              "Enter or Space selects a delegation. Escape clears selection.",
            ),
            "node.a11yDescription.ariaLiveMessage": ({ direction, x, y }) =>
              t("Moved assignment {direction}. Position: {x}, {y}.", {
                direction:
                  direction === "left"
                    ? t("left")
                    : direction === "right"
                      ? t("right")
                      : direction === "up"
                        ? t("up")
                        : direction === "down"
                          ? t("down")
                          : direction,
                x,
                y,
              }),
            "handle.ariaLabel": t("Connection point"),
            "controls.zoomIn.ariaLabel": t("Zoom in"),
            "controls.zoomOut.ariaLabel": t("Zoom out"),
            "controls.fitView.ariaLabel": t("Fit view"),
            "minimap.ariaLabel": t("Delegation graph"),
          }}
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          proOptions={{ hideAttribution: true }}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onNodeClick={(_, node) => select({ kind: "assignment", id: node.id })}
          nodesConnectable={false}
          edgesReconnectable={false}
          deleteKeyCode={null}
          fitView
          minZoom={0.25}
          maxZoom={1.5}
        >
          <Background />
          <Controls showInteractive={false} />
        </ReactFlow>
      </div>
    </>
  );
}
