import { beforeEach, describe, expect, it } from 'vitest';
import { getSortedFilteredVideos, setStore } from '../store';
import type { VideoItem } from '../types';

const sampleVideos: VideoItem[] = [
  {
    id: 'a',
    path: 'C:/videos/a.mp4',
    folder: 'C:/videos',
    size_bytes: 100,
    mtime: 1,
    duration_ms: 1000,
    width: 1920,
    height: 1080,
    aspect_ratio: 1.777,
    favorite: false,
    thumb_path: null,
  },
  {
    id: 'b',
    path: 'C:/videos/b.mp4',
    folder: 'C:/videos',
    size_bytes: 200,
    mtime: 2,
    duration_ms: 500,
    width: 1280,
    height: 720,
    aspect_ratio: 1.777,
    favorite: true,
    thumb_path: null,
  },
  {
    id: 'c',
    path: 'D:/other/c.mp4',
    folder: 'D:/other',
    size_bytes: 50,
    mtime: 3,
    duration_ms: 1500,
    width: 640,
    height: 480,
    aspect_ratio: 1.333,
    favorite: false,
    thumb_path: null,
  },
];

describe('getSortedFilteredVideos', () => {
  beforeEach(() => {
    setStore('videos', sampleVideos);
    setStore('sortMode', 'size');
    setStore('sortDirection', 'asc');
    setStore('filterFolder', null);
    setStore('filterFavorites', false);
  });

  it('sorts by size ascending', () => {
    const result = getSortedFilteredVideos();
    expect(result.map((v) => v.id)).toEqual(['c', 'a', 'b']);
  });

  it('filters by favorites', () => {
    setStore('filterFavorites', true);
    const result = getSortedFilteredVideos();
    expect(result.map((v) => v.id)).toEqual(['b']);
  });

  it('filters by folder', () => {
    setStore('filterFolder', 'D:/other');
    const result = getSortedFilteredVideos();
    expect(result.map((v) => v.id)).toEqual(['c']);
  });
});
