import { useEffect, useRef, useState } from 'react';
import type { GraphView } from '../lib/demo-source';

type Selected = { id: string; label: string; uri: string } | null;
type Point = { x: number; y: number; z: number; iri: string; label: string };

/** Draws a spatial graph using Canvas 2D, without creating a WebGL context. */
export function Graph3D({ graph, onNodeSelect, mode = '3d' }: {
  graph: GraphView; onNodeSelect: (node: Selected) => void; mode?: '2d' | '3d';
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const camera = useRef({ yaw: 0.4, pitch: 0.2, scale: 1, x: 0, y: 0 });
  const points = useRef<Point[]>([]);
  const projected = useRef<{ x: number; y: number; point: Point }[]>([]);
  const drag = useRef<{ x: number; y: number; moved: boolean; button: number } | null>(null);
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    const count = Math.min(graph.classes.length, 2000);
    points.current = graph.classes.slice(0, count).map((node, i) => {
      const angle = i * 2.399963;
      const radius = 12 * Math.sqrt(i + 1);
      return { iri: node.iri, label: node.label || node.iri.split(/[#/]/).pop() || node.iri,
        x: Math.cos(angle) * radius, y: Math.sin(angle) * radius,
        z: Math.sin(i * 1.73) * Math.min(radius, 180) };
    });
    setRevision(v => v + 1);
    const worker = new Worker(new URL('./graph-layout.worker.ts', import.meta.url), { type: 'module' });
    worker.onmessage = (event: MessageEvent<Point[]>) => {
      points.current = event.data;
      setRevision(v => v + 1);
      worker.terminate();
    };
    worker.postMessage({ nodes: points.current, edges: graph.edges.slice(0, 8000) });
    return () => worker.terminate();
  }, [graph]);

  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    const draw = () => {
      const rect = el.getBoundingClientRect();
      const ratio = window.devicePixelRatio || 1;
      el.width = Math.max(1, Math.round(rect.width * ratio));
      el.height = Math.max(1, Math.round(rect.height * ratio));
      const ctx = el.getContext('2d');
      if (!ctx) return;
      ctx.scale(ratio, ratio);
      ctx.fillStyle = '#020617'; ctx.fillRect(0, 0, rect.width, rect.height);
      const c = camera.current;
      const cos = Math.cos(c.yaw), sin = Math.sin(c.yaw);
      const cp = Math.cos(c.pitch), sp = Math.sin(c.pitch);
      const mapped = points.current.map(point => {
        const x = mode === '3d' ? point.x * cos - point.z * sin : point.x;
        const z = mode === '3d' ? point.x * sin + point.z * cos : 0;
        const y = mode === '3d' ? point.y * cp - z * sp : point.y;
        const depth = mode === '3d' ? point.y * sp + z * cp : 0;
        const perspective = 700 / Math.max(250, 700 + depth);
        return { point, depth, x: rect.width / 2 + c.x + x * c.scale * perspective,
          y: rect.height / 2 + c.y + y * c.scale * perspective };
      });
      projected.current = mapped;
      const byId = new Map(mapped.map(p => [p.point.iri, p]));
      for (const edge of graph.edges.slice(0, 8000)) {
        const a = byId.get(edge.source), b = byId.get(edge.target);
        if (!a || !b) continue;
        ctx.strokeStyle = edge.warrant === 'rejected' ? '#fb3b53' : edge.warrant === 'certified' ? '#34d399' : '#475569';
        ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
      }
      mapped.sort((a, b) => b.depth - a.depth);
      for (const p of mapped) {
        ctx.fillStyle = '#7dd3fc'; ctx.beginPath(); ctx.arc(p.x, p.y, 4, 0, Math.PI * 2); ctx.fill();
        if (mapped.length <= 100) { ctx.fillStyle = '#e2e8f0'; ctx.font = '11px sans-serif'; ctx.fillText(p.point.label, p.x + 6, p.y); }
      }
    };
    const observer = new ResizeObserver(draw); observer.observe(el); draw();
    return () => observer.disconnect();
  }, [graph, mode, revision]);

  return <div className="absolute inset-0" style={{ background: '#020617' }}>
    <canvas ref={canvas} style={{ width: '100%', height: '100%', touchAction: 'none' }}
      onContextMenu={e => e.preventDefault()}
      onPointerDown={e => { e.currentTarget.setPointerCapture(e.pointerId); drag.current = { x: e.clientX, y: e.clientY, moved: false, button: e.button }; }}
      onPointerMove={e => { const d = drag.current; if (!d) return; const dx = e.clientX - d.x, dy = e.clientY - d.y;
        if (Math.abs(dx) + Math.abs(dy) > 1) d.moved = true;
        if (mode === '3d' && d.button === 0) { camera.current.yaw += dx * .01; camera.current.pitch += dy * .01; }
        else { camera.current.x += dx; camera.current.y += dy; }
        d.x = e.clientX; d.y = e.clientY; setRevision(v => v + 1); }}
      onPointerUp={e => { const d = drag.current; drag.current = null; if (!d || d.moved) return;
        const rect = e.currentTarget.getBoundingClientRect(); const x = e.clientX - rect.left, y = e.clientY - rect.top;
        const hit = [...projected.current].reverse().find(p => Math.hypot(p.x - x, p.y - y) < 10);
        onNodeSelect(hit ? { id: hit.point.iri.split(/[#/]/).pop() || hit.point.iri, label: hit.point.label, uri: hit.point.iri } : null); }}
      onWheel={e => { camera.current.scale = Math.max(.1, Math.min(5, camera.current.scale * Math.exp(-e.deltaY * .001))); setRevision(v => v + 1); }} />
    <div className="absolute bottom-2 left-2 text-xs text-slate-400 pointer-events-none">{points.current.length} of {graph.classes.length} nodes · {Math.min(graph.edges.length, 8000)} of {graph.edges.length} edges</div>
  </div>;
}
