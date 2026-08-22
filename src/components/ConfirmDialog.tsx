import { useEffect } from "react";
import type { ImageInfo } from "../types";
import { formatBytes } from "../lib/format";

interface Props {
  images: ImageInfo[];
  permanent: boolean;
  busy: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({ images, permanent, busy, onConfirm, onCancel }: Props) {
  const keep = images.length > 0 ? images[0] : null;
  const totalSize = images.reduce((acc, i) => acc + i.size_bytes, 0);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !busy) onCancel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [busy, onCancel]);

  return (
    <div className="viewer-backdrop confirm-backdrop" onClick={busy ? undefined : onCancel}>
      <div className="confirm-dialog" onClick={(e) => e.stopPropagation()} role="dialog">
        <div className="confirm-head">
          <strong>
            {permanent
              ? `Borrar permanentemente ${images.length} archivo(s)`
              : `Enviar ${images.length} archivo(s) a la papelera`}
          </strong>
        </div>

        <div className="confirm-body">
          {keep && (
            <div className="confirm-keep">
              <span className="confirm-label">Se conservará</span>
              <span className="confirm-file">{keep.file_name}</span>
              <span className="muted">{dimsTxt(keep)} · {keep.dir}</span>
            </div>
          )}
          <div className="confirm-delete-list">
            <span className="confirm-label danger-text">
              Se {permanent ? "borrará" : "enviará a la papelera"} ({formatBytes(totalSize)})
            </span>
            <ul>
              {images.map((img) => (
                <li key={img.path} title={img.path}>
                  {img.file_name}
                  <span className="muted"> · {img.dir}</span>
                </li>
              ))}
            </ul>
          </div>
          {permanent && (
            <p className="confirm-warning">
              Esta acción no se puede deshacer.
            </p>
          )}
        </div>

        <div className="confirm-actions">
          <button className="btn" onClick={onCancel} disabled={busy}>
            Cancelar
          </button>
          <button
            className={`btn ${permanent ? "danger-strong" : "danger"}`}
            onClick={onConfirm}
            disabled={busy}
          >
            {busy ? "Borrando..." : permanent ? "Borrar permanentemente" : "Enviar a la papelera"}
          </button>
        </div>
      </div>
    </div>
  );
}

function dimsTxt(img: ImageInfo): string {
  return img.width != null && img.height != null ? `${img.width}x${img.height}` : "?x?";
}
