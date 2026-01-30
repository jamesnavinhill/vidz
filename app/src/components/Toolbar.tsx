import { For, Show, createMemo } from 'solid-js';
import { store, setStore, getUniqueFolders, setAutoplay, setDensity } from '../store';
import { SortMode } from '../types';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import SettingsPanel from './SettingsPanel';

export default function Toolbar() {
  const folders = createMemo(() => getUniqueFolders());

  const addFolder = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select video folder',
    });

    if (selected && typeof selected === 'string') {
      setStore('scanning', true);
      await invoke('add_watched_folder', { path: selected });
    }
  };

  const cancelScan = async () => {
    await invoke('cancel_scan');
  };

  const sortOptions: { value: SortMode; label: string }[] = [
    { value: 'folder', label: 'Folder' },
    { value: 'size', label: 'File Size' },
    { value: 'resolution', label: 'Resolution' },
    { value: 'aspect', label: 'Aspect Ratio' },
    { value: 'duration', label: 'Duration' },
    { value: 'favorites', label: 'Favorites' },
  ];

  return (
    <div
      class="toolbar"
      style={{
        display: 'flex',
        'align-items': 'center',
        gap: '12px',
        padding: '8px 16px',
        background: '#1a1a1a',
        'border-bottom': '1px solid #333',
        'flex-shrink': 0,
      }}
    >
      <button
        onClick={addFolder}
        disabled={store.scanning}
        style={{
          padding: '6px 12px',
          background: '#2d5af7',
          border: 'none',
          'border-radius': '4px',
          color: '#fff',
          cursor: store.scanning ? 'wait' : 'pointer',
          opacity: store.scanning ? 0.7 : 1,
        }}
      >
        {store.scanning ? 'Scanning...' : 'Add Folder'}
      </button>
      <Show when={store.scanning}>
        <button
          onClick={cancelScan}
          style={{
            padding: '6px 10px',
            background: 'transparent',
            border: '1px solid #444',
            'border-radius': '4px',
            color: '#aaa',
            cursor: 'pointer',
          }}
        >
          Cancel
        </button>
      </Show>

      <div style={{ display: 'flex', 'align-items': 'center', gap: '6px' }}>
        <label style={{ color: '#888', 'font-size': '13px' }}>Sort:</label>
        <select
          value={store.sortMode}
          onChange={(e) => setStore('sortMode', e.target.value as SortMode)}
          style={{
            padding: '4px 8px',
            background: '#2a2a2a',
            border: '1px solid #444',
            'border-radius': '4px',
            color: '#fff',
          }}
        >
          <For each={sortOptions}>{(opt) => <option value={opt.value}>{opt.label}</option>}</For>
        </select>
        <button
          onClick={() => setStore('sortDirection', (d) => (d === 'asc' ? 'desc' : 'asc'))}
          style={{
            padding: '4px 8px',
            background: '#2a2a2a',
            border: '1px solid #444',
            'border-radius': '4px',
            color: '#fff',
            cursor: 'pointer',
          }}
        >
          {store.sortDirection === 'asc' ? '↑' : '↓'}
        </button>
      </div>

      <Show when={folders().length > 0}>
        <div style={{ display: 'flex', 'align-items': 'center', gap: '6px' }}>
          <label style={{ color: '#888', 'font-size': '13px' }}>Folder:</label>
          <select
            value={store.filterFolder ?? ''}
            onChange={(e) => setStore('filterFolder', e.target.value || null)}
            style={{
              padding: '4px 8px',
              background: '#2a2a2a',
              border: '1px solid #444',
              'border-radius': '4px',
              color: '#fff',
              'max-width': '200px',
            }}
          >
            <option value="">All folders</option>
            <For each={folders()}>
              {(folder) => {
                const name = folder.split(/[/\\]/).pop() ?? folder;
                return <option value={folder}>{name}</option>;
              }}
            </For>
          </select>
        </div>
      </Show>

      <label
        style={{
          display: 'flex',
          'align-items': 'center',
          gap: '6px',
          color: '#888',
          'font-size': '13px',
          cursor: 'pointer',
        }}
      >
        <input
          type="checkbox"
          checked={store.filterFavorites}
          onChange={(e) => setStore('filterFavorites', e.target.checked)}
        />
        Favorites only
      </label>

      <div style={{ flex: 1 }} />

      <Show when={store.scanCancelled}>
        <div style={{ color: '#b08d57', 'font-size': '12px' }}>Scan cancelled</div>
      </Show>

      <Show when={store.lastWarning}>
        <div style={{ color: '#b08d57', 'font-size': '12px', 'max-width': '160px' }}>
          {store.lastWarning}
        </div>
      </Show>

      <label
        style={{
          display: 'flex',
          'align-items': 'center',
          gap: '6px',
          color: '#888',
          'font-size': '13px',
          cursor: 'pointer',
        }}
      >
        <input
          type="checkbox"
          checked={store.autoplay}
          onChange={(e) => setAutoplay(e.target.checked)}
        />
        Autoplay
      </label>

      <div style={{ display: 'flex', 'align-items': 'center', gap: '6px' }}>
        <label style={{ color: '#888', 'font-size': '13px' }}>Columns: {store.gridColumns}</label>
        <input
          type="range"
          min="0.5"
          max="2"
          step="0.1"
          value={store.density}
          onInput={(e) => setDensity(parseFloat(e.target.value))}
          style={{ width: '80px' }}
        />
      </div>

      <div style={{ color: '#666', 'font-size': '12px' }}>{store.videos.length} videos</div>

      <SettingsPanel />
    </div>
  );
}
