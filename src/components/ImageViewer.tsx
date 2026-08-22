import { useEffect } from "react";
import type { ImageInfo } from "../types";
import { convertFileSrc } from "@tauri-apps/api/core";
import { formatBytes, dims } from "../lib/format";

interface Props {
  images: ImageInfo[];
  index: number;
  onClose: () => void;
  onNavigate: (index: number) => void;
}

export function ImageViewer({ images, index, onClose, onNavigate }: Props) {
  const image = images[index];

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
      else if (e.key === "ArrowLeft" && index > 0) onNavigate(index - 1);
      else if (e.key === "ArrowRight" && index < images.length - 1) onNavigate(index + 1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [index, images.length, onClose, onNavigate]);

  if (!image) return null;

  return (
    <div className="viewer-backdrop" onClick={onClose}>
      <div className="viewer" onClick={(e) => e.stopPropagation()}>
        <div className="viewer-head">
          <span>
            {image.file_name}
            <span className="muted"> · {index + 1} de {images.length}</span>
          </span>
          <div className="viewer-nav">
            <button
              className="btn"
              onClick={() => onNavigate(index - 1)}
              disabled={index === 0}
              title="Anterior (←)"
            >
              ←
            </button>
            <button
              className="btn"
              onClick={() => onNavigate(index + 1)}
              disabled={index === images.length - 1}
              title="Siguiente (→)"
            >
              →
            </button>
            <button className="btn" onClick={onClose}>
              Cerrar
            </button>
          </div>
        </div>
        <div className="viewer-body">
          <img src={convertFileSrc(image.path)} alt={image.file_name} />
        </div>
        <div className="viewer-meta">
          <span>
            {dims(image.width, image.height)} · {formatBytes(image.size_bytes)}
          </span>
          <span>{image.date_taken ?? image.modified ?? "sin fecha"}</span>
          {image.camera && <span>{image.camera}</span>}
          <span className="muted">{image.dir}</span>
        </div>
      </div>
    </div>
  );
}
