import { useCallback, useEffect, useRef, useState } from "react";
import type { ImageInfo } from "../types";
import { convertFileSrc } from "@tauri-apps/api/core";
import { formatBytes, dims } from "../lib/format";

const MIN_ZOOM = 1;
const MAX_ZOOM = 12;
const ZOOM_STEP = 1.25;

interface Props {
  images: ImageInfo[];
  initialLeft: number;
  initialRight: number;
  onClose: () => void;
}

interface Transform {
  zoom: number;
  x: number;
  y: number;
}

const IDENTITY: Transform = { zoom: 1, x: 0, y: 0 };

export function CompareViewer({ images, initialLeft, initialRight, onClose }: Props) {
  const [leftIdx, setLeftIdx] = useState(initialLeft);
  const [rightIdx, setRightIdx] = useState(initialRight === initialLeft ? (initialLeft + 1) % images.length : initialRight);
  const [transform, setTransform] = useState<Transform>(IDENTITY);
  const [dragging, setDragging] = useState(false);
  const dragStart = useRef<{ mx: number; my: number; tx: number; ty: number } | null>(null);
  const paneRefs = useRef<(HTMLDivElement | null)[]>([]);
  const canvasRefs = useRef<(HTMLDivElement | null)[]>([]);

  const zoomBy = useCallback((factor: number) => {
    setTransform((t) => ({
      zoom: Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, t.zoom * factor)),
      x: t.x,
      y: t.y,
    }));
  }, []);

  const left = images[leftIdx];
  const right = images[rightIdx];

  const resetView = useCallback(() => setTransform(IDENTITY), []);

  const stepPair = useCallback(
    (dir: 1 | -1) => {
      let a = leftIdx;
      let b = rightIdx;
      if (dir === 1) {
        b += 1;
        if (b >= images.length) {
          a += 1;
          b = a + 1;
        }
      } else {
        b -= 1;
        if (b <= a) {
          a -= 1;
          b = images.length - 1;
        }
      }
      if (a < 0 || b >= images.length || a >= b) return;
      setLeftIdx(a);
      setRightIdx(b);
      resetView();
    },
    [leftIdx, rightIdx, images.length, resetView]
  );

  const canPrev = !(leftIdx === 0 && rightIdx === 1);
  const canNext = !(rightIdx === images.length - 1 && leftIdx === images.length - 2);

  const onPointerDown = (e: React.PointerEvent) => {
    setDragging(true);
    dragStart.current = { mx: e.clientX, my: e.clientY, tx: transform.x, ty: transform.y };
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  };

  const onPointerMove = (e: React.PointerEvent) => {
    if (!dragging || !dragStart.current) return;
    const s = dragStart.current;
    setTransform((t) => ({ ...t, x: s.tx + (e.clientX - s.mx), y: s.ty + (e.clientY - s.my) }));
  };

  const endDrag = () => {
    setDragging(false);
    dragStart.current = null;
  };

  useEffect(() => {
    const handlers = canvasRefs.current.map((canvas) => {
      const fn = (e: WheelEvent) => {
        e.preventDefault();
        zoomBy(e.deltaY < 0 ? ZOOM_STEP : 1 / ZOOM_STEP);
      };
      canvas?.addEventListener("wheel", fn, { passive: false });
      return [canvas, fn] as const;
    });
    return () => {
      for (const [canvas, fn] of handlers) canvas?.removeEventListener("wheel", fn);
    };
  }, [zoomBy]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      else if (e.key === "ArrowLeft") stepPair(-1);
      else if (e.key === "ArrowRight") stepPair(1);
      else if (e.key === "+" || e.key === "=") zoomBy(ZOOM_STEP);
      else if (e.key === "-") zoomBy(1 / ZOOM_STEP);
      else if (e.key === "0") resetView();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose, stepPair, resetView, zoomBy]);

  useEffect(() => {
    for (const pane of paneRefs.current) {
      if (pane) pane.scrollTop = 0;
    }
  }, [leftIdx, rightIdx]);

  if (!left || !right) return null;

  const renderPane = (img: ImageInfo, idx: number, side: "izq" | "der", i: number) => (
    <div className="compare-pane" ref={(el) => (paneRefs.current[i] = el)}>
      <div className="compare-pane-head">
        <span className={`side-tag ${side}`}>{side}</span>
        <select
          value={idx}
          onChange={(e) => {
            side === "izq" ? setLeftIdx(Number(e.target.value)) : setRightIdx(Number(e.target.value));
            resetView();
          }}
        >
          {images.map((im, k) => (
            <option key={im.path} value={k}>
              {k + 1}. {im.file_name}
            </option>
          ))}
        </select>
      </div>
      <div
        className={`compare-canvas ${dragging ? "dragging" : ""}`}
        ref={(el) => (canvasRefs.current[i] = el)}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endDrag}
        onPointerLeave={endDrag}
        onDoubleClick={resetView}
      >
        <img
          src={convertFileSrc(img.path)}
          alt={img.file_name}
          draggable={false}
          style={{
            transform: `translate(${transform.x}px, ${transform.y}px) scale(${transform.zoom})`,
          }}
        />
      </div>
      <div className="compare-pane-meta">
        <span>{dims(img.width, img.height)}</span>
        <span>{formatBytes(img.size_bytes)}</span>
        <span>{img.date_taken ?? img.modified ?? "sin fecha"}</span>
      </div>
    </div>
  );

  return (
    <div className="viewer-backdrop" onClick={onClose}>
      <div className="compare-viewer" onClick={(e) => e.stopPropagation()}>
        <div className="viewer-head">
          <span>
            Comparar {leftIdx + 1} y {rightIdx + 1}
            <span className="muted"> de {images.length}</span>
          </span>
          <div className="viewer-nav">
            <button className="btn" onClick={() => stepPair(-1)} disabled={!canPrev} title="Par anterior (←)">
              ←
            </button>
            <button className="btn" onClick={() => stepPair(1)} disabled={!canNext} title="Par siguiente (→)">
              →
            </button>
            <button className="btn" onClick={() => zoomBy(ZOOM_STEP)} title="Acercar (+)">
              +
            </button>
            <button className="btn" onClick={() => zoomBy(1 / ZOOM_STEP)} title="Alejar (-)">
              −
            </button>
            <button className="btn" onClick={resetView} title="Restablecer vista (0)">
              Ajustar
            </button>
            <button className="btn" onClick={onClose}>
              Cerrar
            </button>
          </div>
        </div>
        <div className="compare-body">
          {renderPane(left, leftIdx, "izq", 0)}
          {renderPane(right, rightIdx, "der", 1)}
        </div>
        <div className="compare-help">
          Rueda: zoom sincronizado · Arrastrar: mover · Doble clic: ajustar · ←/→: cambiar par · Esc: salir · Zoom {Math.round(transform.zoom * 100)}%
        </div>
      </div>
    </div>
  );
}
