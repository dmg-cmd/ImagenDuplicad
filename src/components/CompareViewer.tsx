import { useCallback, useEffect, useRef, useState } from "react";
import type { ImageInfo } from "../types";
import { convertFileSrc } from "@tauri-apps/api/core";
import { formatBytes, dims } from "../lib/format";

const MIN_ZOOM = 1;
const MAX_ZOOM = 12;
const ZOOM_STEP = 1.25;
const DIFF_ZOOM = 6;
const DIFF_THRESHOLD = 20;
const DIFF_MAX_WIDTH = 1600;

type Mode = "lado" | "cortina" | "fundido" | "diff";

const MODES: { id: Mode; label: string }[] = [
  { id: "lado", label: "Lado a lado" },
  { id: "cortina", label: "Cortina" },
  { id: "fundido", label: "Fundido" },
  { id: "diff", label: "Diferencia" },
];

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

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("no se pudo cargar la imagen"));
    img.src = src;
  });
}

async function computeDiff(srcA: string, srcB: string): Promise<string> {
  const [a, b] = await Promise.all([loadImage(srcA), loadImage(srcB)]);
  const w0 = Math.min(a.naturalWidth, b.naturalWidth);
  const h0 = Math.min(a.naturalHeight, b.naturalHeight);
  if (w0 === 0 || h0 === 0) throw new Error("dimensiones inválidas");
  const cw = Math.min(DIFF_MAX_WIDTH, w0);
  const ch = Math.max(1, Math.round((h0 * cw) / w0));

  const pixelsOf = (img: HTMLImageElement): Uint8ClampedArray => {
    const c = document.createElement("canvas");
    c.width = cw;
    c.height = ch;
    const ctx = c.getContext("2d", { willReadFrequently: true })!;
    ctx.drawImage(img, 0, 0, cw, ch);
    return ctx.getImageData(0, 0, cw, ch).data;
  };

  const pa = pixelsOf(a);
  const pb = pixelsOf(b);
  const out = new ImageData(cw, ch);

  for (let i = 0; i < pa.length; i += 4) {
    const d =
      (Math.abs(pa[i] - pb[i]) +
        Math.abs(pa[i + 1] - pb[i + 1]) +
        Math.abs(pa[i + 2] - pb[i + 2])) /
      3;
    const o = out.data;
    if (d > DIFF_THRESHOLD) {
      const k = Math.min(1, d / 80);
      o[i] = 220 * k + 20;
      o[i + 1] = 15;
      o[i + 2] = 25;
      o[i + 3] = 255;
    } else {
      const lum = 0.299 * pa[i] + 0.587 * pa[i + 1] + 0.114 * pa[i + 2];
      const g = 40 + lum * 0.3;
      o[i] = g;
      o[i + 1] = g;
      o[i + 2] = g;
      o[i + 3] = 255;
    }
  }

  const oc = document.createElement("canvas");
  oc.width = cw;
  oc.height = ch;
  oc.getContext("2d")!.putImageData(out, 0, 0);
  return oc.toDataURL("image/png");
}

export function CompareViewer({ images, initialLeft, initialRight, onClose }: Props) {
  const [leftIdx, setLeftIdx] = useState(initialLeft);
  const [rightIdx, setRightIdx] = useState(
    initialRight === initialLeft ? (initialLeft + 1) % images.length : initialRight
  );
  const [transform, setTransform] = useState<Transform>(IDENTITY);
  const [mode, setMode] = useState<Mode>("lado");
  const [split, setSplit] = useState(50);
  const [opacity, setOpacity] = useState(100);
  const [dragging, setDragging] = useState(false);
  const [diffUrl, setDiffUrl] = useState<string | null>(null);
  const [diffError, setDiffError] = useState<string | null>(null);
  const dragStart = useRef<{ mx: number; my: number; tx: number; ty: number } | null>(null);
  const paneRefs = useRef<(HTMLDivElement | null)[]>([]);
  const canvasRefs = useRef<(HTMLDivElement | null)[]>([]);

  const left = images[leftIdx];
  const right = images[rightIdx];

  const zoomBy = useCallback((factor: number) => {
    setTransform((t) => ({
      zoom: Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, t.zoom * factor)),
      x: t.x,
      y: t.y,
    }));
  }, []);

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

  const changeMode = (m: Mode) => {
    setMode(m);
    resetView();
    setSplit(50);
    setOpacity(100);
  };

  const startPan = (e: React.PointerEvent) => {
    setDragging(true);
    dragStart.current = { mx: e.clientX, my: e.clientY, tx: transform.x, ty: transform.y };
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  };

  const movePan = (e: React.PointerEvent) => {
    if (!dragging || !dragStart.current) return;
    const s = dragStart.current;
    setTransform((t) => ({ ...t, x: s.tx + (e.clientX - s.mx), y: s.ty + (e.clientY - s.my) }));
  };

  const moveSplit = (canvas: HTMLDivElement, clientX: number) => {
    const rect = canvas.getBoundingClientRect();
    const pct = ((clientX - rect.left) / rect.width) * 100;
    setSplit(Math.min(98, Math.max(2, pct)));
  };

  const onCanvasPointerDown = (e: React.PointerEvent, i: number) => {
    const canvas = canvasRefs.current[i];
    if (!canvas) return;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
    if (mode === "cortina") {
      moveSplit(canvas, e.clientX);
      return;
    }
    startPan(e);
  };

  const onCanvasPointerMove = (e: React.PointerEvent, i: number) => {
    if (!dragging) return;
    if (mode === "cortina") {
      const canvas = canvasRefs.current[i];
      if (canvas) moveSplit(canvas, e.clientX);
      return;
    }
    movePan(e);
  };

  const endDrag = () => {
    setDragging(false);
    dragStart.current = null;
  };

  const zoomToPoint = (canvas: HTMLDivElement, clientX: number, clientY: number) => {
    const rect = canvas.getBoundingClientRect();
    const cx = clientX - rect.left - rect.width / 2;
    const cy = clientY - rect.top - rect.height / 2;
    const px = (cx - transform.x) / transform.zoom;
    const py = (cy - transform.y) / transform.zoom;
    const newZoom = Math.min(MAX_ZOOM, Math.max(transform.zoom * 2.5, DIFF_ZOOM));
    setTransform({ zoom: newZoom, x: -newZoom * px, y: -newZoom * py });
  };

  useEffect(() => {
    if (mode !== "diff") {
      setDiffUrl(null);
      setDiffError(null);
      return;
    }
    let alive = true;
    setDiffUrl(null);
    setDiffError(null);
    computeDiff(srcOf(left), srcOf(right))
      .then((url) => alive && setDiffUrl(url))
      .catch(() => alive && setDiffError("No se pudo calcular la diferencia entre estas imágenes"));
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode, leftIdx, rightIdx]);

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
  }, [zoomBy, mode]);

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

  const srcOf = (img: ImageInfo) => convertFileSrc(img.path);

  const pickerFor = (idx: number, side: "izq" | "der") => (
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
  );

  const metaRowFor = (img: ImageInfo, side: "izq" | "der") => (
    <>
      <span className={`side-tag ${side}`}>{side}</span>
      <span>{dims(img.width, img.height)}</span>
      <span>{formatBytes(img.size_bytes)}</span>
      <span>{img.date_taken ?? img.modified ?? "sin fecha"}</span>
    </>
  );

  const overlayStyle: React.CSSProperties =
    mode === "cortina"
      ? { clipPath: `inset(0 ${100 - split}% 0 0)` }
      : { opacity: opacity / 100 };

  const overlayCanvas = (
    <div
      className={`compare-canvas overlay ${dragging && mode === "lado" ? "dragging" : ""}`}
      ref={(el) => (canvasRefs.current[0] = el)}
      onPointerDown={(e) => onCanvasPointerDown(e, 0)}
      onPointerMove={(e) => onCanvasPointerMove(e, 0)}
      onPointerUp={endDrag}
      onPointerLeave={endDrag}
      onDoubleClick={(e) => zoomToPoint(e.currentTarget, e.clientX, e.clientY)}
    >
      <div
        className="compare-stack"
        style={{ transform: `translate(${transform.x}px, ${transform.y}px) scale(${transform.zoom})` }}
      >
        <img src={srcOf(left)} alt={left.file_name} draggable={false} />
        <img
          className="overlay-img"
          src={srcOf(right)}
          alt={right.file_name}
          draggable={false}
          style={overlayStyle}
        />
      </div>
      {mode === "cortina" && (
        <div className="curtain-line" style={{ left: `${split}%` }}>
          <div className="curtain-handle">⇔</div>
        </div>
      )}
    </div>
  );

  return (
    <div className="viewer-backdrop" onClick={onClose}>
      <div className="compare-viewer" onClick={(e) => e.stopPropagation()}>
        <div className="viewer-head">
          <div className="mode-tabs">
            {MODES.map((m) => (
              <button
                key={m.id}
                className={`tab ${mode === m.id ? "active" : ""}`}
                onClick={() => changeMode(m.id)}
              >
                {m.label}
              </button>
            ))}
          </div>
          <div className="viewer-nav">
            <select
              value={leftIdx}
              onChange={(e) => {
                setLeftIdx(Number(e.target.value));
                resetView();
              }}
              title="Imagen izquierda"
            >
              {images.map((im, k) => (
                <option key={im.path} value={k}>
                  {k + 1}. {im.file_name}
                </option>
              ))}
            </select>
            <select
              value={rightIdx}
              onChange={(e) => {
                setRightIdx(Number(e.target.value));
                resetView();
              }}
              title="Imagen derecha"
            >
              {images.map((im, k) => (
                <option key={im.path} value={k}>
                  {k + 1}. {im.file_name}
                </option>
              ))}
            </select>
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

        {mode === "lado" ? (
          <div className="compare-body">
            {[leftIdx, rightIdx].map((idx, i) => {
              const img = images[idx];
              const side = i === 0 ? "izq" : "der";
              return (
                <div className="compare-pane" key={idx} ref={(el) => (paneRefs.current[i] = el)}>
                  <div className="compare-pane-head">
                    <span className={`side-tag ${side}`}>{side}</span>
                    {pickerFor(idx, side)}
                  </div>
                  <div
                    className={`compare-canvas ${dragging ? "dragging" : ""}`}
                    ref={(el) => (canvasRefs.current[i] = el)}
                    onPointerDown={(e) => onCanvasPointerDown(e, i)}
                    onPointerMove={(e) => onCanvasPointerMove(e, i)}
                    onPointerUp={endDrag}
                    onPointerLeave={endDrag}
                    onDoubleClick={(e) => zoomToPoint(e.currentTarget, e.clientX, e.clientY)}
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
                  <div className="compare-pane-meta">{metaRowFor(img, side)}</div>
                </div>
              );
            })}
          </div>
        ) : mode === "diff" ? (
          <div className="compare-body single">
            <div className="compare-pane">
              <div className="compare-pane-head">
                <span className="side-tag izq">{leftIdx + 1}. {left.file_name}</span>
                <span className="side-tag der">{rightIdx + 1}. {right.file_name}</span>
              </div>
              <div
                className={`compare-canvas diff ${dragging ? "dragging" : ""}`}
                ref={(el) => (canvasRefs.current[0] = el)}
                onPointerDown={(e) => onCanvasPointerDown(e, 0)}
                onPointerMove={(e) => onCanvasPointerMove(e, 0)}
                onPointerUp={endDrag}
                onPointerLeave={endDrag}
                onDoubleClick={(e) => zoomToPoint(e.currentTarget, e.clientX, e.clientY)}
              >
                {diffUrl ? (
                  <img
                    className="diff-img"
                    src={diffUrl}
                    alt="Mapa de diferencias"
                    draggable={false}
                    style={{
                      transform: `translate(${transform.x}px, ${transform.y}px) scale(${transform.zoom})`,
                    }}
                  />
                ) : diffError ? (
                  <div className="no-thumb">{diffError}</div>
                ) : (
                  <div className="no-thumb">Calculando diferencias...</div>
                )}
              </div>
              <div className="compare-pane-meta">
                <span className="diff-legend">🔴 Zonas distintas · ⬛ Zonas iguales (atenuadas)</span>
              </div>
            </div>
          </div>
        ) : (
          <div className="compare-body single">
            <div className="compare-pane">
              <div className="compare-pane-head">
                <span className="side-tag izq">{leftIdx + 1}. {left.file_name}</span>
                <span className="side-tag der">{rightIdx + 1}. {right.file_name}</span>
              </div>
              {overlayCanvas}
              <div className="compare-pane-meta">
                {metaRowFor(left, "izq")}
              </div>
              <div className="compare-pane-meta">
                {metaRowFor(right, "der")}
              </div>
            </div>
          </div>
        )}

        {mode === "fundido" && (
          <div className="fade-row">
            <span>Opacidad imagen superior ({rightIdx + 1})</span>
            <input
              type="range"
              min={0}
              max={100}
              value={opacity}
              onChange={(e) => setOpacity(Number(e.target.value))}
            />
            <span className="pct">{opacity}%</span>
          </div>
        )}

        <div className="compare-help">
          {mode === "lado"
            ? "Rueda: zoom sincronizado · Arrastrar: mover · Doble clic: zoom al punto · ←/→: cambiar par · Esc: salir"
            : mode === "cortina"
              ? "Arrastrar: mover la cortina · Rueda: zoom sincronizado · Doble clic: zoom al punto · Esc: salir"
              : mode === "fundido"
                ? "Mueve el control de opacidad para hacer el fundido · Rueda: zoom · Doble clic: zoom al punto · Esc: salir"
                : "Rojo = píxeles distintos entre ambas imágenes · Rueda: zoom · Arrastrar: mover · Doble clic: zoom al punto"}
          {" · "}
          Zoom {Math.round(transform.zoom * 100)}%
        </div>
      </div>
    </div>
  );
}
