import { createSignal, onCleanup, createEffect, onMount } from 'solid-js';
import { VideoItem } from '../types';
import { convertFileSrc } from '@tauri-apps/api/core';
import { canVideoPlay, registerPlaying, unregisterPlaying } from '../store';

interface Props {
  video: VideoItem;
  width: number;
  isActive: boolean;
  autoplay: boolean;
  onSelect: () => void;
}

export default function VideoTile(props: Props) {
  let videoRef: HTMLVideoElement | undefined;
  let containerRef: HTMLDivElement | undefined;

  const [isHovering, setIsHovering] = createSignal(false);
  const [isVisible, setIsVisible] = createSignal(false);
  const [isPlaying, setIsPlaying] = createSignal(false);

  onMount(() => {
    if (!containerRef) return;

    const observer = new IntersectionObserver(
      (entries) => {
        setIsVisible(entries[0]?.isIntersecting ?? false);
      },
      { threshold: 0.3 },
    );

    observer.observe(containerRef);
    onCleanup(() => observer.disconnect());
  });

  const shouldPlay = () => {
    if (!props.isActive) return false;
    if (!isVisible()) return false;
    if (props.autoplay) return true;
    return isHovering();
  };

  createEffect(() => {
    const wantsToPlay = shouldPlay();
    const allowed = wantsToPlay && canVideoPlay(props.video.id);

    if (allowed && !isPlaying() && videoRef) {
      registerPlaying(props.video.id);
      setIsPlaying(true);
      videoRef.play().catch(() => {});
    } else if (!wantsToPlay && isPlaying() && videoRef) {
      unregisterPlaying(props.video.id);
      setIsPlaying(false);
      videoRef.pause();
      videoRef.currentTime = 0;
    }
  });

  onCleanup(() => {
    if (isPlaying()) {
      unregisterPlaying(props.video.id);
    }
  });

  const videoSrc = () => convertFileSrc(props.video.path);

  const aspectRatio = () => props.video.aspect_ratio ?? 16 / 9;
  const height = () => props.width / aspectRatio();

  return (
    <div
      ref={containerRef}
      class="video-tile"
      style={{
        width: `${props.width}px`,
        height: `${height()}px`,
        position: 'relative',
        cursor: 'pointer',
        overflow: 'hidden',
        'background-color': '#1a1a1a',
        'flex-shrink': 0,
      }}
      onMouseEnter={() => setIsHovering(true)}
      onMouseLeave={() => setIsHovering(false)}
      onClick={() => props.onSelect()}
    >
      <video
        ref={videoRef}
        src={videoSrc()}
        muted
        loop
        playsinline
        preload="auto"
        style={{
          width: '100%',
          height: '100%',
          'object-fit': 'cover',
          position: 'absolute',
          top: 0,
          left: 0,
          'background-color': '#1a1a1a',
        }}
      />

      {props.video.favorite && (
        <div
          style={{
            position: 'absolute',
            top: '4px',
            right: '4px',
            color: '#ff4757',
            'font-size': '16px',
            'z-index': 1,
          }}
        >
          ♥
        </div>
      )}
    </div>
  );
}
