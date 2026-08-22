import { useEffect, useState } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import type { DupGroup, ImageInfo } from "../types";
import { formatBytes, dims } from "../lib/format";
import { enqueue, useInView } from "../lib/thumbQueue";
import { ImageViewer } from "./ImageViewer";
import { ConfirmDialog } from "./ConfirmDialog";

const thumbCache = new Map<string, string>();

function Thumb({ image, onOpen }: { image: ImageInfo; onOpen: () => void }) {
  const [thumb, setThumb] = useState<string | null>(thumbCache.get(image.path) ?? null);
  const [failed, setFailed] = useState(false);
  const { ref, inView } = useInView<HTMLDivElement>();

  useEffect(() => {
    if (!inView || thumb || failed) return;
    let alive = true;
    enqueue(() => invoke<string>("preview", { path: image.path }))
      .then((t) => {
        thumbCache.set(image.path, t);
        if (alive) setThumb(t);
      })
      .catch(() => {
        if (alive) setFailed(true);
      });
    return () => {
      alive = false;
    };
  }, [inView, image.path, thumb, failed]);

  return (
    <div className="thumb" ref={ref}>
      {thumb ? (
        <img
          src={convertFileSrc(thumb)}
          alt={image.file_name}
          onClick={onOpen}
          title="Ver en grande"
        />
      ) : (
        <div className="no-thumb">{failed ? "Sin vista previa" : "Cargando..."}</div>
      )}
    </div>
  );
}

interface Props {
  group: DupGroup;
  onDeleted: (deleted: Set<string>) => void;
}

export function GroupCard({ group, onDeleted }: Props) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [previewIndex, setPreviewIndex] = useState<number | null>(null);
  const [confirming, setConfirming] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  const toggle = (path: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  };

  const selectAll = () => setSelected(new Set(group.images.map((i) => i.path)));
  const clearAll = () => setSelected(new Set());
  const keepBest = () => {
    const best = group.images[0].path;
    setSelected(new Set(group.images.filter((i) => i.path !== best).map((i) => i.path)));
  };

  const toDelete = () =>
    group.images.filter((img) => selected.has(img.path));

  const doDelete = async (permanent: boolean) => {
    const paths = [...selected];
    if (paths.length === 0) return;
    setBusy(true);
    setError(null);
    try {
      if (permanent) {
        await invoke("delete_permanent", { req: { paths } });
      } else {
        await invoke("delete_to_trash", { req: { paths } });
      }
      onDeleted(new Set(paths));
      setSelected(new Set());
      setConfirming(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const keeping = selected.size === group.images.length - 1;

  return (
    <section className={`group ${group.confidence}`}>
      <div className="group-head">
        <div className="group-title">
          <span className={`badge ${group.confidence}`}>
            {group.confidence === "exacto" ? "Exacto" : "Probable"}
          </span>
          <strong>{group.images.length} imágenes</strong>
          <span className="muted">({formatBytes(group.total_size)})</span>
          {keeping && <span className="keep-note">Conservarás 1 archivo</span>}
        </div>
        <div className="group-actions">
          <button onClick={keepBest} className="btn">
            Conservar la mejor
          </button>
          <button onClick={selectAll} className="btn">
            Seleccionar todas
          </button>
          <button onClick={clearAll} className="btn" disabled={selected.size === 0}>
            Limpiar
          </button>
          <button
            onClick={() => setConfirming(false)}
            className="btn danger"
            disabled={selected.size === 0 || busy}
          >
            Borrar seleccionadas ({selected.size})
          </button>
          <button
            onClick={() => setConfirming(true)}
            className="btn danger-strong"
            disabled={selected.size === 0 || busy}
          >
            Borrar permanentemente
          </button>
        </div>
      </div>

      {error && <div className="group-error">{error}</div>}

      <div className="image-row">
        {group.images.map((img, idx) => (
          <div
            key={img.path}
            className={`image-card ${selected.has(img.path) ? "selected" : ""}`}
            onClick={() => toggle(img.path)}
            title={img.path}
          >
            <div onClick={(e) => e.stopPropagation()}>
              <Thumb image={img} onOpen={() => setPreviewIndex(idx)} />
            </div>
            <div className="info">
              <div className="fname">{img.file_name}</div>
              <div className="fdir" title={img.dir}>
                📁 {img.dir}
              </div>
              <div className="meta">
                {img.width && <span>{dims(img.width, img.height)}</span>}
                <span>{formatBytes(img.size_bytes)}</span>
                <span>{img.date_taken ?? img.modified ?? "sin fecha"}</span>
                {img.camera && <span>{img.camera}</span>}
              </div>
            </div>
            <div className="check" />
          </div>
        ))}
      </div>

      {previewIndex !== null && (
        <ImageViewer
          images={group.images}
          index={previewIndex}
          onClose={() => setPreviewIndex(null)}
          onNavigate={setPreviewIndex}
        />
      )}

      {confirming !== null && (
        <ConfirmDialog
          images={toDelete()}
          permanent={confirming}
          busy={busy}
          onConfirm={() => doDelete(confirming)}
          onCancel={() => !busy && setConfirming(null)}
        />
      )}
    </section>
  );
}
