import { createSignal, createEffect, onCleanup, For, createMemo, Show } from 'solid-js';
import { createVirtualizer } from '@tanstack/solid-virtual';
import { store, setStore, getSortedFilteredVideos } from '../store';
import VideoTile from './VideoTile';

export default function VideoGrid() {
  let containerRef: HTMLDivElement | undefined;
  const [containerWidth, setContainerWidth] = createSignal(800);
  const [containerHeight, setContainerHeight] = createSignal(0);
  const [scrollTop, setScrollTop] = createSignal(0);

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

  const rowHeights = createMemo(() => {
    const list = videos();
    const cols = columnCount();
    const width = itemSize();
    const heights: number[] = [];

    for (let i = 0; i < list.length; i += cols) {
      let maxHeight = 0;
      for (let j = 0; j < cols && i + j < list.length; j += 1) {
        const aspect = list[i + j]?.aspect_ratio ?? 16 / 9;
        const height = width / aspect;
        if (height > maxHeight) maxHeight = height;
      }
      heights.push(maxHeight + gap);
    }

    return heights;
  });

  const rowCount = createMemo(() => rowHeights().length);

  const getRowHeight = (rowIndex: number) => {
    const heights = rowHeights();
    const fallback = itemSize() / (16 / 9) + gap;
    return heights[rowIndex] ?? fallback;
  };

  const virtualizer = createMemo(() =>
    createVirtualizer({
      count: rowCount(),
      getScrollElement: () => containerRef ?? null,
      estimateSize: (index) => getRowHeight(index),
      overscan: 4,
    }),
  );

  createEffect(() => {
    if (!containerRef) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerWidth(entry.contentRect.width);
        setContainerHeight(entry.contentRect.height);
      }
    });

    observer.observe(containerRef);
    onCleanup(() => observer.disconnect());
  });

  const virtualRows = createMemo(() => virtualizer().getVirtualItems());
  const totalSize = createMemo(() => virtualizer().getTotalSize());
  const viewportBottom = createMemo(() => scrollTop() + containerHeight());

  createEffect(() => {
    rowHeights();
    virtualizer().measure();
  });

  const isRowInViewport = (rowStart: number, rowSize: number) => {
    const top = scrollTop();
    const bottom = viewportBottom();
    const rowEnd = rowStart + rowSize;
    return rowEnd >= top && rowStart <= bottom;
  };

  const autoplayAllowance = createMemo(() => {
    if (!store.autoplay || store.focusedId !== null) {
      return new Set<string>();
    }

    const cols = columnCount();
    const list = videos();
    const allowed = new Set<string>();
    const maxAllowed = Math.max(1, store.maxConcurrentVideos);

    for (const row of virtualRows()) {
      if (!isRowInViewport(row.start, row.size)) continue;

      const startIndex = row.index * cols;
      for (let i = 0; i < cols; i += 1) {
        const video = list[startIndex + i];
        if (!video) continue;
        allowed.add(video.id);
        if (allowed.size >= maxAllowed) {
          return allowed;
        }
      }
    }

    return allowed;
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
      onScroll={() => {
        if (!containerRef) return;

        setScrollTop(containerRef.scrollTop);

        if (!store.toolbarManualCollapsed) {
          setStore('toolbarCollapsed', true);
        }
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
            const rowVisible = isRowInViewport(virtualRow.start, virtualRow.size);

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
                          inViewport={rowVisible}
                          allowAutoplay={autoplayAllowance().has(video!.id)}
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
