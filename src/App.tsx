import { onCleanup, onMount, Show } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { store, setStore, upsertVideos, removeVideo, removeVideos } from './store';
import { VideoItem, ScanProgress, AppSettings } from './types';
import VideoGrid from './components/VideoGrid';
import Toolbar from './components/Toolbar';
import FocusedPlayer from './components/FocusedPlayer';
import WarningToast from './components/WarningToast';
import './App.css';

function App() {
  onMount(async () => {
    const [videos, settings, folders] = await Promise.all([
      invoke<VideoItem[]>('get_library'),
      invoke<AppSettings>('get_app_settings'),
      invoke<string[]>('get_watched_folders'),
    ]);
    setStore('videos', videos);
    setStore('autoplay', settings.autoplay);
    setStore('density', settings.density);
    setStore('watchedFolders', folders);

    let pendingUpdates: VideoItem[] = [];
    let flushTimeout: number | null = null;

    const flushUpdates = () => {
      flushTimeout = null;
      if (pendingUpdates.length === 0) return;
      const map = new Map(pendingUpdates.map((video) => [video.id, video]));
      pendingUpdates = [];
      upsertVideos(Array.from(map.values()));
    };

    const scheduleFlush = () => {
      if (flushTimeout !== null) return;
      flushTimeout = window.setTimeout(flushUpdates, 120);
    };

    const unlistenDiscovered = await listen<VideoItem[]>('library:discovered', (event) => {
      pendingUpdates.push(...event.payload);
      scheduleFlush();
    });

    const unlistenUpdated = await listen<VideoItem[]>('library:updated', (event) => {
      pendingUpdates.push(...event.payload);
      scheduleFlush();
    });

    const unlistenRemoved = await listen<string>('library:removed', (event) => {
      removeVideo(event.payload);
    });

    const unlistenRemovedBulk = await listen<string[]>('library:removed_bulk', (event) => {
      removeVideos(event.payload);
    });

    const unlistenScanProgress = await listen<ScanProgress>('library:scan_progress', (event) => {
      setStore('scanProgress', {
        total: event.payload.total,
        processed: event.payload.processed,
      });
    });

    const unlistenScanFinished = await listen('library:scan_finished', () => {
      setStore('scanProgress', null);
      setStore('scanning', false);
    });

    const unlistenWarnings = await listen<string>('library:warning', (event) => {
      setStore('lastWarning', event.payload);
      window.clearTimeout(store.warningTimeoutId);
      const timeoutId = window.setTimeout(() => {
        setStore('lastWarning', null);
      }, 4500);
      setStore('warningTimeoutId', timeoutId);
    });

    const unlistenCancelled = await listen('library:scan_cancelled', () => {
      setStore('scanCancelled', true);
      setStore('scanning', false);
      window.setTimeout(() => setStore('scanCancelled', false), 3000);
    });

    const unlistenWatchedFolders = await listen<string[]>(
      'library:watched_folders_updated',
      (event) => {
        setStore('watchedFolders', event.payload);
      },
    );

    await invoke('start_file_watcher');

    if (videos.length > 0) {
      invoke('process_pending_jobs');
    }

    onCleanup(() => {
      window.clearTimeout(store.warningTimeoutId);
      unlistenDiscovered();
      unlistenUpdated();
      unlistenRemoved();
      unlistenRemovedBulk();
      unlistenScanProgress();
      unlistenScanFinished();
      unlistenWarnings();
      unlistenCancelled();
      unlistenWatchedFolders();
    });
  });

  return (
    <div
      class="app"
      style={{
        display: 'flex',
        'flex-direction': 'column',
        height: '100vh',
        width: '100vw',
        background: '#0d0d0d',
        color: '#fff',
        overflow: 'hidden',
      }}
    >
      <Toolbar />

      <Show
        when={store.videos.length > 0}
        fallback={
          <div
            style={{
              flex: 1,
              display: 'flex',
              'flex-direction': 'column',
              'align-items': 'center',
              'justify-content': 'center',
              color: '#666',
            }}
          >
            <Show when={store.scanning}>
              <div style={{ 'font-size': '18px', 'margin-bottom': '12px' }}>Scanning...</div>
              <Show when={store.scanProgress}>
                <div style={{ 'font-size': '14px' }}>
                  {store.scanProgress!.processed} / {store.scanProgress!.total} files
                </div>
              </Show>
            </Show>
            <Show when={!store.scanning}>
              <div style={{ 'font-size': '18px', 'margin-bottom': '8px' }}>No videos yet</div>
              <div style={{ 'font-size': '14px' }}>Click "Add Folder" to get started</div>
            </Show>
          </div>
        }
      >
        <VideoGrid />
      </Show>

      <FocusedPlayer />
      <WarningToast />
    </div>
  );
}

export default App;
