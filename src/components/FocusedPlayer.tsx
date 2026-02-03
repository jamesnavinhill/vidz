import { Show, createMemo } from 'solid-js';
import { store, setStore } from '../store';
import { invoke } from '@tauri-apps/api/core';
import { convertFileSrc } from '@tauri-apps/api/core';

export default function FocusedPlayer() {
  const video = createMemo(() => store.videos.find((v) => v.id === store.focusedId));

  const close = () => setStore('focusedId', null);

  const toggleFavorite = async () => {
    const v = video();
    if (!v) return;
    await invoke('set_favorite', { id: v.id, favorite: !v.favorite });
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      close();
    }
  };

  return (
    <Show when={video()}>
      <div
        class="focused-player-overlay"
        style={{
          position: 'fixed',
          inset: 0,
          background: 'rgba(0, 0, 0, 0.95)',
          display: 'flex',
          'flex-direction': 'column',
          'align-items': 'center',
          'justify-content': 'center',
          'z-index': 1000,
        }}
        onClick={(e) => {
          if (e.target === e.currentTarget) close();
        }}
        onKeyDown={handleKeyDown}
        tabIndex={0}
      >
        <div
          style={{
            position: 'absolute',
            top: '16px',
            right: '16px',
            display: 'flex',
            gap: '12px',
          }}
        >
          <button
            onClick={toggleFavorite}
            style={{
              background: 'transparent',
              border: 'none',
              color: video()!.favorite ? '#ff4757' : '#888',
              'font-size': '24px',
              cursor: 'pointer',
            }}
          >
            ♥
          </button>
          <button
            onClick={close}
            style={{
              background: 'transparent',
              border: 'none',
              color: '#fff',
              'font-size': '24px',
              cursor: 'pointer',
            }}
          >
            ✕
          </button>
        </div>

        <video
          src={convertFileSrc(video()!.path)}
          controls
          autoplay
          style={{
            'max-width': '90vw',
            'max-height': '85vh',
            outline: 'none',
          }}
        />
      </div>
    </Show>
  );
}
