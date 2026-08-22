export interface ImageInfo {
  path: string;
  file_name: string;
  dir: string;
  size_bytes: number;
  modified: string | null;
  date_taken: string | null;
  camera: string | null;
  width: number | null;
  height: number | null;
  hash: string;
  thumbnail: string | null;
}

export interface DupGroup {
  confidence: "exacto" | "probable";
  images: ImageInfo[];
  total_size: number;
}

export interface ScanProgress {
  phase: string;
  done: number;
  total: number;
  detail: string | null;
}

export interface ScanResult {
  groups: DupGroup[];
  skipped: number;
}