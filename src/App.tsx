import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DupGroup, ImageInfo, ScanProgress, ScanResult } from "./types";
import { GroupCard } from "./components/GroupCard";

type UIGroup = DupGroup & { id: string };

let groupIdCounter = 0;
const nextGroupId = () => `g${++groupIdCounter}`;

function compareImages(a: ImageInfo, b: ImageInfo) {
  const pa = (a.width ?? 0) * (a.height ?? 0);
  const pb = (b.width ?? 0) * (b.height ?? 0);
  return (
    pb - pa ||
    (a.date_taken ?? "").localeCompare(b.date_taken ?? "") ||
    a.path.localeCompare(b.path)
  );
}

function buildGroup(images: ImageInfo[], confidence: string): UIGroup {
  const sorted = [...images].sort(compareImages);
  return {
    id: nextGroupId(),
    confidence: confidence as DupGroup["confidence"],
    images: sorted,
    total_size: sorted.reduce((acc, i) => acc + i.size_bytes, 0),
  };
}

function mergeGroups(prev: UIGroup[], incoming: DupGroup, excluded: Set<string>): UIGroup[] {
  if (incoming.images.some((i) => excluded.has(i.path))) return prev;
  const idx = prev.findIndex((g) =>
    g.images.some((i) => incoming.images.some((ii) => ii.path === i.path)),
  );
  if (idx === -1) {
    const merged = buildGroup(incoming.images, incoming.confidence);
    // Los grupos nuevos arriba para verlos en cuanto aparecen
    return [merged, ...prev];
  }
  const byPath = new Map(prev[idx].images.map((i) => [i.path, i]));
  for (const img of incoming.images) byPath.set(img.path, img);
  const updated = buildGroup([...byPath.values()], incoming.confidence);
  const next = [...prev];
  next[idx] = updated;
  return next;
}

export default function App() {
  const [groups, setGroups] = useState<UIGroup[]>([]);
  const [skipped, setSkipped] = useState(0);
  const [scanning, setScanning] = useState(false);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [folder, setFolder] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [buscarSimilares, setBuscarSimilares] = useState(false);
  const [umbral, setUmbral] = useState(8);
  const [visibleGroups, setVisibleGroups] = useState(50);
  const deletedRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    setVisibleGroups(50);
  }, [groups]);

  useEffect(() => {
    let unlistenProgress: UnlistenFn | undefined;
    let unlistenDup: UnlistenFn | undefined;
    (async () => {
      unlistenProgress = await listen<ScanProgress>("scan-progress", (e) => {
        setProgress(e.payload);
      });
      unlistenDup = await listen<DupGroup>("dup-found", (e) => {
        setGroups((prev) => mergeGroups(prev, e.payload, deletedRef.current));
      });
    })();

    const params = new URLSearchParams(window.location.search);
    const autoDir = params.get("auto");
    let timer: number | undefined;
    if (autoDir) {
      timer = window.setTimeout(() => {
        setFolder(autoDir);
        runScan(autoDir);
      }, 500);
    }

    return () => {
      unlistenProgress?.();
      unlistenDup?.();
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, []);

  const pickFolder = async () => {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") {
      setFolder(dir);
      await runScan(dir);
    }
  };

  const runScan = async (dir: string) => {
    setScanning(true);
    setError(null);
    setGroups([]);
    setSkipped(0);
    setProgress(null);
    deletedRef.current = new Set();
    try {
      const result = await invoke<ScanResult>("scan_folder", {
        dir,
        buscarSimilares,
        umbral,
      });
      const remaining = result.groups.filter(
        (g) =>
          g.images.length > 1 &&
          !g.images.some((i) => deletedRef.current.has(i.path)),
      );
      setGroups(remaining.map((g) => buildGroup(g.images, g.confidence)));
      setSkipped(result.skipped);
    } catch (e) {
      const msg = String(e);
      if (msg !== "Escaneo cancelado") {
        setError(msg);
      }
    } finally {
      setScanning(false);
    }
  };

  const cancelScan = async () => {
    try {
      await invoke("cancel_scan");
    } catch {
      // ignorar
    }
  };

  const removeDeleted = (deletedPaths: Set<string>) => {
    for (const p of deletedPaths) deletedRef.current.add(p);
    setGroups((prev) =>
      prev
        .map((g) => ({
          ...g,
          images: g.images.filter((i) => !deletedPaths.has(i.path)),
        }))
        .filter((g) => g.images.length > 1),
    );
  };

  const total = groups.reduce((acc, g) => acc + g.images.length, 0);

  return (
    <div className="app">
      <header className="topbar">
        <h1>Imagen Duplicada</h1>
        <button onClick={pickFolder} disabled={scanning} className="btn primary">
          {scanning ? "Escaneando..." : "Escanear carpeta"}
        </button>
        {scanning && (
          <button onClick={cancelScan} className="btn cancel">
            Cancelar
          </button>
        )}
        {folder && <span className="folder">{folder}</span>}
        <button
          className="btn"
          onClick={() => invoke("abrir_historial").catch((e) => setError(String(e)))}
          title="Abrir el registro CSV de imágenes borradas"
        >
          📄 Historial
        </button>
        <label className="scan-option" title="Compara por contenido visual aunque cambien el tamaño, dimensiones o fecha (más lento)">
          <input
            type="checkbox"
            checked={buscarSimilares}
            onChange={(e) => setBuscarSimilares(e.target.checked)}
            disabled={scanning}
          />
          Buscar similares
        </label>
        <select
          className="scan-similitud"
          value={umbral}
          onChange={(e) => setUmbral(Number(e.target.value))}
          disabled={scanning || !buscarSimilares}
          title="Nivel de exigencia para considerar dos imágenes similares"
        >
          <option value={4}>Estricta</option>
          <option value={8}>Normal</option>
          <option value={12}>Laxa</option>
          <option value={16}>Muy laxa</option>
        </select>
      </header>

      {scanning && progress && (
        <div className="progress-section">
          <div className="progress-phase">{progress.phase}</div>
          <div className="progress-track">
            <div
              className="progress-bar"
              style={{
                width: `${progress.total ? (progress.done / progress.total) * 100 : 0}%`,
              }}
            />
          </div>
          <div className="progress-info">
            <span className="progress-count">
              {progress.done}/{progress.total}
            </span>
            {progress.detail && (
              <span className="progress-detail">{progress.detail}</span>
            )}
          </div>
        </div>
      )}

      {error && <div className="error">{error}</div>}

      {!scanning && groups.length === 0 && !error && (
        <div className="empty">
          <p>
            {folder
              ? "No se encontraron imágenes duplicadas en esa carpeta."
              : "Selecciona una carpeta para buscar imágenes duplicadas por hash y metadatos."}
          </p>
        </div>
      )}

      {groups.length > 0 && (
        <div className="summary">
          <strong>{groups.length}</strong> grupo(s) de duplicados ·{" "}
          <strong>{total}</strong> imágenes
          {scanning && <span className="skipped-note"> · escaneo en curso…</span>}
          {!scanning && skipped > 0 && (
            <span className="skipped-note"> · {skipped} archivo(s) no legibles omitidos</span>
          )}
          {groups.length > visibleGroups && (
            <span className="skipped-note"> · mostrando {visibleGroups}</span>
          )}
        </div>
      )}

      <main className="groups">
        {groups.slice(0, visibleGroups).map((g) => (
          <GroupCard key={g.id} group={g} onDeleted={removeDeleted} />
        ))}
      </main>

      {groups.length > visibleGroups && (
        <div className="load-more">
          <button className="btn primary" onClick={() => setVisibleGroups((v) => v + 100)}>
            Mostrar más grupos ({groups.length - visibleGroups} restantes)
          </button>
        </div>
      )}
    </div>
  );
}
