import { createStore } from 'solid-js/store';
import { invoke } from '@tauri-apps/api/core';
import { VideoItem, SortMode, SortDirection, AppSettings } from './types';

export interface AppStore {
  videos: VideoItem[];
  autoplay: boolean;
  sortMode: SortMode;
  sortDirection: SortDirection;
  filterFolder: string | null;
  filterFavorites: boolean;
  focusedId: string | null;
  density: number;
  scanning: boolean;
  scanProgress: { total: number; processed: number } | null;
  activePlayingIds: Set<string>;
  maxConcurrentVideos: number;
  lastWarning: string | null;
  warningTimeoutId: number;
  scanCancelled: boolean;
}

const [store, setStore] = createStore<AppStore>({
  videos: [],
  autoplay: true,
  sortMode: 'folder',
  sortDirection: 'asc',
  filterFolder: null,
  filterFavorites: false,
  focusedId: null,
  density: 1,
  scanning: false,
  scanProgress: null,
  activePlayingIds: new Set(),
  maxConcurrentVideos: 16,
  lastWarning: null,
  warningTimeoutId: 0,
  scanCancelled: false,
});

export { store, setStore };

export function upsertVideos(newVideos: VideoItem[]) {
  setStore('videos', (videos) => {
    const map = new Map(videos.map((v) => [v.id, v]));
    for (const video of newVideos) {
      map.set(video.id, video);
    }
    return Array.from(map.values());
  });
}

export function removeVideo(id: string) {
  setStore('videos', (videos) => videos.filter((v) => v.id !== id));
}

export function removeVideos(ids: string[]) {
  if (ids.length === 0) return;
  const idSet = new Set(ids);
  setStore('videos', (videos) => videos.filter((v) => !idSet.has(v.id)));
}

export function getSortedFilteredVideos(): VideoItem[] {
  let result = [...store.videos];

  if (store.filterFolder) {
    result = result.filter((v) => v.folder === store.filterFolder);
  }
  if (store.filterFavorites) {
    result = result.filter((v) => v.favorite);
  }

  const dir = store.sortDirection === 'asc' ? 1 : -1;

  result.sort((a, b) => {
    let cmp = 0;
    switch (store.sortMode) {
      case 'size':
        cmp = a.size_bytes - b.size_bytes;
        break;
      case 'resolution':
        cmp = ((a.width ?? 0) * (a.height ?? 0)) - ((b.width ?? 0) * (b.height ?? 0));
        break;
      case 'aspect':
        cmp = (a.aspect_ratio ?? 0) - (b.aspect_ratio ?? 0);
        break;
      case 'duration':
        cmp = (a.duration_ms ?? 0) - (b.duration_ms ?? 0);
        break;
      case 'folder':
        cmp = a.folder.localeCompare(b.folder);
        break;
      case 'favorites':
        cmp = (b.favorite ? 1 : 0) - (a.favorite ? 1 : 0);
        break;
    }
    if (cmp === 0) {
      cmp = a.path.localeCompare(b.path);
    }
    return cmp * dir;
  });

  return result;
}

export function getUniqueFolders(): string[] {
  const folders = new Set(store.videos.map((v) => v.folder));
  return Array.from(folders).sort();
}

export function canVideoPlay(id: string): boolean {
  if (store.activePlayingIds.has(id)) {
    return true;
  }
  return store.activePlayingIds.size < store.maxConcurrentVideos;
}

export function registerPlaying(id: string) {
  setStore('activePlayingIds', (set) => {
    const newSet = new Set(set);
    newSet.add(id);
    return newSet;
  });
}

export function unregisterPlaying(id: string) {
  setStore('activePlayingIds', (set) => {
    const newSet = new Set(set);
    newSet.delete(id);
    return newSet;
  });
}

export function setAutoplay(value: boolean) {
  setStore('autoplay', value);
  const { density } = store;
  persistSettings(value, density);
}

export function setDensity(value: number) {
  setStore('density', value);
  const { autoplay } = store;
  persistSettings(autoplay, value);
}

function persistSettings(autoplay: boolean, density: number) {
  const settings: AppSettings = {
    autoplay,
    density,
  };
  invoke('save_app_settings', { settings }).catch(console.error);
}
