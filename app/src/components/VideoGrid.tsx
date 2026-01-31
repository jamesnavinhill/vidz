import { createSignal, createEffect, onCleanup, For, createMemo, Show } from 'solid-js';
import { createVirtualizer } from '@tanstack/solid-virtual';
import { store, setStore, getSortedFilteredVideos } from '../store';
import VideoTile from './VideoTile';

export default function VideoGrid() {
  let containerRef: HTMLDivElement | undefined;
  const [containerWidth, setContainerWidth] = createSignal(800);

  const baseSize = 200;
  const gap = 4;

  const columnCount = createMemo(() => {
    const size = baseSize * store.density;
    return Math.max(1, Math.floor((containerWidth() + gap) / (size + gap)));
  });

  createEffect(() => {
    setStore('gridColumns', columnCount());
  });

  const itemSize = createMemo(() => {
    const cols = columnCount();
    return (containerWidth() - gap * (cols - 1)) / cols;
  });

  const videos = createMemo(() => getSortedFilteredVideos());

  const rowCount = createMemo(() => Math.ceil(videos().length / columnCount()));

  const getRowHeight = (rowIndex: number) => {
    const cols = columnCount();
    const startIndex = rowIndex * cols;
    const rowVideos = videos().slice(startIndex, startIndex + cols);
    const width = itemSize();

    let maxHeight = 0;
    for (const video of rowVideos) {
      const aspect = video.aspect_ratio ?? 16 / 9;
      const height = width / aspect;
      if (height > maxHeight) maxHeight = height;
    }
    return maxHeight + gap;
  };

  const virtualizer = createMemo(() =>
    createVirtualizer({
      count: rowCount(),
      getScrollElement: () => containerRef ?? null,
      estimateSize: (index) => getRowHeight(index),
      overscan: 3,
    }),
  );

  createEffect(() => {
    if (!containerRef) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerWidth(entry.contentRect.width);
      }
    });

    observer.observe(containerRef);
    onCleanup(() => observer.disconnect());
  });

  const virtualRows = createMemo(() => virtualizer().getVirtualItems());
  const totalSize = createMemo(() => virtualizer().getTotalSize());

  createEffect(() => {
    videos();
    columnCount();
    virtualizer().measure();
  });

  return (
    <div
      ref={containerRef}
      class="video-grid-container"
      style={{
        height: '100%',
        width: '100%',
        overflow: 'auto',
      }}
    >
      <div
        style={{
          height: `${totalSize()}px`,
          width: '100%',
          position: 'relative',
        }}
      >
        <For each={virtualRows()}>
          {(virtualRow) => {
            const cols = columnCount();
            const size = itemSize();
            const startIndex = virtualRow.index * cols;

            return (
              <div
                style={{
                  position: 'absolute',
                  top: `${virtualRow.start}px`,
                  left: 0,
                  width: '100%',
                  height: `${virtualRow.size - gap}px`,
                  display: 'flex',
                  'align-items': 'flex-end',
                  gap: `${gap}px`,
                }}
              >
                <For each={Array.from({ length: cols }, (_, i) => startIndex + i)}>
                  {(index) => {
                    const video = videos()[index];
                    return (
                      <Show when={video}>
                        <VideoTile
                          video={video!}
                          width={size}
                          isActive={store.focusedId === null}
                          autoplay={store.autoplay}
                          onSelect={() => setStore('focusedId', video!.id)}
                        />
                      </Show>
                    );
                  }}
                </For>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}
