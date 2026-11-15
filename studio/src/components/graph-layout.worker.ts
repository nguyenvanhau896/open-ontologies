/// <reference lib="webworker" />
export {};
type Node = { x: number; y: number; z: number; iri: string; label: string };
self.onmessage = (event: MessageEvent<{ nodes: Node[]; edges: { source: string; target: string }[] }>) => {
  const { nodes, edges } = event.data;
  const byId = new Map(nodes.map(n => [n.iri, n]));
  // Bounded spring relaxation. Keep initial spacing without quadratic repulsion.
  const anchors = nodes.map(n => ({ ...n }));
  for (let tick = 0; tick < 120; tick++) {
    for (const edge of edges) {
      const a = byId.get(edge.source), b = byId.get(edge.target);
      if (!a || !b) continue;
      const dx = b.x - a.x, dy = b.y - a.y, dz = b.z - a.z;
      const length = Math.hypot(dx, dy, dz) || 1;
      const force = Math.max(-2, Math.min(2, (length - 60) * .015));
      a.x += dx / length * force; a.y += dy / length * force; a.z += dz / length * force;
      b.x -= dx / length * force; b.y -= dy / length * force; b.z -= dz / length * force;
    }
    nodes.forEach((n, i) => { n.x += (anchors[i].x - n.x) * .004; n.y += (anchors[i].y - n.y) * .004; });
  }
  self.postMessage(nodes);
};
