import { createSignal, For, Show, onMount } from 'solid-js';
import { invoke } from '@tauri-apps/api/core';
import { store, setStore, setAutoplay } from '../store';

export default function SettingsPanel() {
  const [isOpen, setIsOpen] = createSignal(false);
  const [watchedFolders, setWatchedFolders] = createSignal<string[]>([]);
  
  onMount(async () => {
    const folders = await invoke<string[]>('get_watched_folders');
    setWatchedFolders(folders);
  });
  
  const removeFolder = async (path: string) => {
    await invoke('remove_watched_folder', { path });
    setWatchedFolders((folders) => folders.filter((f) => f !== path));
  };
  
  const getFolderName = (path: string) => {
    return path.split(/[/\\]/).pop() ?? path;
  };
  
  return (
    <>
      <button
        onClick={() => setIsOpen(true)}
        style={{
          padding: '6px 12px',
          background: '#2a2a2a',
          border: '1px solid #444',
          'border-radius': '4px',
          color: '#fff',
          cursor: 'pointer',
        }}
      >
        ⚙
      </button>
      
      <Show when={isOpen()}>
        <div
          style={{
            position: 'fixed',
            inset: 0,
            background: 'rgba(0, 0, 0, 0.8)',
            display: 'flex',
            'align-items': 'center',
            'justify-content': 'center',
            'z-index': 1000,
          }}
          onClick={(e) => {
            if (e.target === e.currentTarget) setIsOpen(false);
          }}
        >
          <div
            style={{
              background: '#1a1a1a',
              'border-radius': '8px',
              padding: '24px',
              'min-width': '400px',
              'max-width': '600px',
              'max-height': '80vh',
              overflow: 'auto',
            }}
          >
            <div style={{ display: 'flex', 'justify-content': 'space-between', 'margin-bottom': '20px' }}>
              <h2 style={{ margin: 0, 'font-size': '18px' }}>Settings</h2>
              <button
                onClick={() => setIsOpen(false)}
                style={{
                  background: 'transparent',
                  border: 'none',
                  color: '#888',
                  'font-size': '18px',
                  cursor: 'pointer',
                }}
              >
                ✕
              </button>
            </div>
            
            <div style={{ 'margin-bottom': '20px' }}>
              <h3 style={{ 'font-size': '14px', color: '#888', 'margin-bottom': '12px' }}>Playback</h3>
              <label style={{ display: 'flex', 'align-items': 'center', gap: '8px', cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={store.autoplay}
                  onChange={(e) => setAutoplay(e.target.checked)}
                />
                Autoplay videos on hover
              </label>
            </div>
            
            <div style={{ 'margin-bottom': '20px' }}>
              <h3 style={{ 'font-size': '14px', color: '#888', 'margin-bottom': '12px' }}>Performance</h3>
              <label style={{ display: 'flex', 'align-items': 'center', gap: '8px' }}>
                Max concurrent videos:
                <input
                  type="number"
                  min="4"
                  max="32"
                  value={store.maxConcurrentVideos}
                  onChange={(e) => setStore('maxConcurrentVideos', parseInt(e.target.value) || 16)}
                  style={{
                    width: '60px',
                    padding: '4px 8px',
                    background: '#2a2a2a',
                    border: '1px solid #444',
                    'border-radius': '4px',
                    color: '#fff',
                  }}
                />
              </label>
            </div>
            
            <div>
              <h3 style={{ 'font-size': '14px', color: '#888', 'margin-bottom': '12px' }}>Watched Folders</h3>
              <Show
                when={watchedFolders().length > 0}
                fallback={<div style={{ color: '#666' }}>No folders added yet</div>}
              >
                <For each={watchedFolders()}>
                  {(folder) => (
                    <div
                      style={{
                        display: 'flex',
                        'align-items': 'center',
                        'justify-content': 'space-between',
                        padding: '8px 12px',
                        background: '#2a2a2a',
                        'border-radius': '4px',
                        'margin-bottom': '8px',
                      }}
                    >
                      <div>
                        <div style={{ 'font-size': '14px' }}>{getFolderName(folder)}</div>
                        <div style={{ 'font-size': '11px', color: '#666' }}>{folder}</div>
                      </div>
                      <button
                        onClick={() => removeFolder(folder)}
                        style={{
                          background: 'transparent',
                          border: 'none',
                          color: '#ff4757',
                          cursor: 'pointer',
                          'font-size': '14px',
                        }}
                      >
                        Remove
                      </button>
                    </div>
                  )}
                </For>
              </Show>
            </div>
          </div>
        </div>
      </Show>
    </>
  );
}
