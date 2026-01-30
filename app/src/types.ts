export interface VideoItem {
  id: string;
  path: string;
  folder: string;
  size_bytes: number;
  mtime: number;
  duration_ms: number | null;
  width: number | null;
  height: number | null;
  aspect_ratio: number | null;
  favorite: boolean;
  thumb_path: string | null;
}

export interface ScanProgress {
  total: number;
  processed: number;
  current_file: string | null;
}

export type SortMode = 
  | 'size' 
  | 'resolution' 
  | 'aspect' 
  | 'duration' 
  | 'folder' 
  | 'favorites';

export type SortDirection = 'asc' | 'desc';

export interface AppSettings {
  autoplay: boolean;
  density: number;
}
