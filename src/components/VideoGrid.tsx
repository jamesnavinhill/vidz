import { createSignal, createEffect, onCleanup, For, createMemo, Show } from 'solid-js';
import { createVirtualizer } from '@tanstack/solid-virtual';
import { invoke } from '@tauri-apps/api/core';
import { store, setStore, getSortedFilteredVideos } from '../store';
import { VideoItem } from '../types';
import VideoTile from './VideoTile';

const DECODE_HEAVY_CODECS = new Set([
  'av1',
  'vp9',
  'hevc',
  'h265',
  'prores',
  'dnxhd',
  'dnxhr',
]);

const DENSE_POOL_MIN_COLUMNS = 7;
const WARMUP_IDLE_DELAY_MS = 260;

function isDecodeHeavy(video: VideoItem): boolean {
  const codec = (video.codec_name ?? '').toLowerCase();
  if (codec && DECODE_HEAVY_CODECS.has(codec)) {
    return true;
  }

  const width = video.width ?? 0;
  const height = video.height ?? 0;
  return width * height >= 2560 * 1440;
}

export default function VideoGrid() {
  let containerRef: HTMLDivElement | undefined;
  const [containerWidth, setContainerWidth] = createSignal(800);
  const [containerHeight, setContainerHeight] = createSignal(0);
  const [scrollTop, setScrollTop] = createSignal(0);
  const [scrollVelocity, setScrollVelocity] = createSignal(0);
  const [scrollDirection, setScrollDirection] = createSignal<'up' | 'down'>('down');
  const [isScrolling, setIsScrolling] = createSignal(false);
  const [idleWarmupIds, setIdleWarmupIds] = createSignal<Set<string>>(new Set());
  let lastScrollAt = 0;
  let lastScrollPosition = 0;
  let scrollIdleTimeout: number | null = null;
  let hintsTimeout: number | null = null;
  let warmupTimeout: number | null = null;

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

  const overscan = createMemo(() => {
    const velocity = scrollVelocity();
    if (velocity > 2.2) return 12;
    if (velocity > 1.0) return 8;
    return 4;
  });

  const virtualizer = createMemo(() =>
    createVirtualizer({
      count: rowCount(),
      getScrollElement: () => containerRef ?? null,
      estimateSize: (index) => getRowHeight(index),
      overscan: overscan(),
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

  const decodeDropActive = createMemo(
    () =>
      store.autoplay &&
      store.focusedId === null &&
      isScrolling() &&
      (scrollVelocity() > 1.2 || columnCount() >= DENSE_POOL_MIN_COLUMNS),
  );

  const autoplayAllowance = createMemo(() => {
    if (!store.autoplay || store.focusedId !== null) {
      return new Set<string>();
    }

    const cols = columnCount();
    const list = videos();
    const visible: VideoItem[] = [];
    const maxAllowed = Math.max(1, store.maxConcurrentVideos);

    for (const row of virtualRows()) {
      if (!isRowInViewport(row.start, row.size)) continue;
      const startIndex = row.index * cols;
      for (let i = 0; i < cols; i += 1) {
        const video = list[startIndex + i];
        if (video) {
          visible.push(video);
        }
      }
    }

    const allowed = new Set<string>();
    if (!decodeDropActive()) {
      for (const video of visible) {
        allowed.add(video.id);
        if (allowed.size >= maxAllowed) {
          break;
        }
      }
      return allowed;
    }

    const heavyBudget = Math.max(1, Math.floor(maxAllowed * 0.25));
    let heavyUsed = 0;

    for (const video of visible) {
      if (allowed.size >= maxAllowed) break;
      if (!isDecodeHeavy(video)) {
        allowed.add(video.id);
      }
    }

    for (const video of visible) {
      if (allowed.size >= maxAllowed) break;
      if (!isDecodeHeavy(video) || allowed.has(video.id)) continue;
      if (heavyUsed >= heavyBudget) continue;
      allowed.add(video.id);
      heavyUsed += 1;
    }

    if (allowed.size === 0 && visible.length > 0) {
      allowed.add(visible[0].id);
    }

    return allowed;
  });

  const prefetchIds = createMemo(() => {
    const rows = virtualRows();
    if (rows.length === 0) return new Set<string>();

    const cols = columnCount();
    const list = videos();
    const ids = new Set<string>();
    const windowRows = scrollVelocity() > 1.5 ? 4 : 2;
    const direction = scrollDirection();

    const ordered = [...rows].sort((a, b) =>
      direction === 'down' ? a.index - b.index : b.index - a.index,
    );

    for (const row of ordered) {
      const rowVisible = isRowInViewport(row.start, row.size);
      const rowAhead =
        direction === 'down' ? row.start > viewportBottom() : row.start + row.size < scrollTop();
      if (rowVisible || !rowAhead) continue;

      const startIndex = row.index * cols;
      for (let i = 0; i < cols; i += 1) {
        const video = list[startIndex + i];
        if (!video) continue;
        if (decodeDropActive() && isDecodeHeavy(video)) continue;
        ids.add(video.id);
        if (ids.size >= store.maxConcurrentVideos * 2) {
          return ids;
        }
      }

      if (ids.size > 0 && ids.size >= windowRows * cols) {
        break;
      }
    }

    return ids;
  });

  const warmupCandidates = createMemo(() => {
    if (!store.autoplay || store.focusedId !== null || isScrolling()) {
      return [];
    }

    const cols = columnCount();
    const list = videos();
    const ids: string[] = [];
    const maxWarmup = Math.max(store.maxConcurrentVideos, cols * 2);
    const viewportTop = scrollTop();
    const viewportEnd = viewportBottom();
    const warmupBand = Math.max(containerHeight() * 0.8, itemSize() * 2);
    const autoplaySet = autoplayAllowance();
    const prefetchSet = prefetchIds();

    for (const row of virtualRows()) {
      const rowVisible = isRowInViewport(row.start, row.size);
      if (rowVisible) continue;

      const rowCenter = row.start + row.size / 2;
      if (rowCenter < viewportTop - warmupBand || rowCenter > viewportEnd + warmupBand) {
        continue;
      }

      const startIndex = row.index * cols;
      for (let i = 0; i < cols; i += 1) {
        const video = list[startIndex + i];
        if (!video) continue;
        if (decodeDropActive() && isDecodeHeavy(video)) continue;

        const id = video.id;
        if (autoplaySet.has(id) || prefetchSet.has(id)) continue;
        ids.push(id);
        if (ids.length >= maxWarmup) {
          return ids;
        }
      }
    }

    return ids;
  });

  createEffect(() => {
    if (warmupTimeout !== null) {
      window.clearTimeout(warmupTimeout);
      warmupTimeout = null;
    }

    const candidates = warmupCandidates();
    if (candidates.length === 0) {
      setIdleWarmupIds(new Set<string>());
      return;
    }

    warmupTimeout = window.setTimeout(() => {
      setIdleWarmupIds(new Set(candidates));
      warmupTimeout = null;
    }, WARMUP_IDLE_DELAY_MS);
  });

  createEffect(() => {
    if (isScrolling() && idleWarmupIds().size > 0) {
      setIdleWarmupIds(new Set<string>());
    }
  });

  const densePoolingActive = createMemo(
    () => store.autoplay && store.focusedId === null && columnCount() >= DENSE_POOL_MIN_COLUMNS,
  );

  const pooledMountIds = createMemo<Set<string> | null>(() => {
    if (!densePoolingActive()) {
      return null;
    }

    const budget = Math.min(
      120,
      Math.max(store.maxConcurrentVideos * 2, Math.floor(columnCount() * 2.5)),
    );

    const ids = new Set<string>();

    for (const id of autoplayAllowance()) {
      ids.add(id);
      if (ids.size >= budget) return ids;
    }

    for (const id of prefetchIds()) {
      ids.add(id);
      if (ids.size >= budget) return ids;
    }

    for (const id of idleWarmupIds()) {
      ids.add(id);
      if (ids.size >= budget) return ids;
    }

    return ids;
  });

  const priorityIds = createMemo(() => {
    const ids = new Set<string>();
    const cols = columnCount();
    const list = videos();

    for (const row of virtualRows()) {
      if (!isRowInViewport(row.start, row.size)) continue;
      const startIndex = row.index * cols;
      for (let i = 0; i < cols; i += 1) {
        const video = list[startIndex + i];
        if (!video) continue;
        ids.add(video.id);
      }
    }

    for (const id of prefetchIds()) {
      ids.add(id);
      if (ids.size >= 256) return Array.from(ids);
    }

    for (const id of idleWarmupIds()) {
      ids.add(id);
      if (ids.size >= 256) return Array.from(ids);
    }

    return Array.from(ids);
  });

  createEffect(() => {
    const payload = {
      priority_ids: priorityIds(),
      is_scrolling: isScrolling(),
      scroll_velocity: scrollVelocity(),
      estimated_tile_width: Math.round(itemSize()),
    };

    if (hintsTimeout !== null) {
      window.clearTimeout(hintsTimeout);
    }

    hintsTimeout = window.setTimeout(() => {
      hintsTimeout = null;
      invoke('update_ui_activity', { hints: payload }).catch(console.error);
    }, 120);
  });

  onCleanup(() => {
    if (scrollIdleTimeout !== null) window.clearTimeout(scrollIdleTimeout);
    if (hintsTimeout !== null) window.clearTimeout(hintsTimeout);
    if (warmupTimeout !== null) window.clearTimeout(warmupTimeout);
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

        const now = performance.now();
        const top = containerRef.scrollTop;
        const delta = top - lastScrollPosition;
        const elapsed = now - lastScrollAt;

        if (elapsed > 0) {
          const velocity = Math.abs(delta) / elapsed;
          setScrollVelocity(velocity);
          if (delta > 0) setScrollDirection('down');
          if (delta < 0) setScrollDirection('up');
        }

        lastScrollAt = now;
        lastScrollPosition = top;
        setScrollTop(top);
        setIsScrolling(true);
        if (scrollIdleTimeout !== null) window.clearTimeout(scrollIdleTimeout);
        scrollIdleTimeout = window.setTimeout(() => {
          setIsScrolling(false);
          setScrollVelocity(0);
        }, 180);

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
            const pool = pooledMountIds();
            const autoplaySet = autoplayAllowance();
            const prefetchSet = prefetchIds();
            const warmupSet = idleWarmupIds();

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
                          allowAutoplay={autoplaySet.has(video!.id)}
                          allowPrefetch={prefetchSet.has(video!.id)}
                          allowWarmup={warmupSet.has(video!.id)}
                          allowVideoMount={pool ? pool.has(video!.id) : true}
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
