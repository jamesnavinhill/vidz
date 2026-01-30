import { Show } from 'solid-js';
import { store } from '../store';

export default function WarningToast() {
  return (
    <Show when={store.lastWarning}>
      <div
        style={{
          position: 'absolute',
          bottom: '16px',
          right: '16px',
          background: 'rgba(18, 18, 18, 0.9)',
          border: '1px solid #333',
          'border-radius': '6px',
          padding: '10px 12px',
          color: '#c7a46a',
          'font-size': '12px',
          'max-width': '320px',
        }}
      >
        {store.lastWarning}
      </div>
    </Show>
  );
}
