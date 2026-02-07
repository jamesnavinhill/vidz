import { createSignal, createEffect, createMemo, Show } from 'solid-js';
import { VideoItem } from '../types';
import { convertFileSrc } from '@tauri-apps/api/core';

interface Props {
  video: VideoItem;
  width: number;
  isActive: boolean;
  inViewport: boolean;
  allowAutoplay: boolean;
  autoplay: boolean;
  onSelect: () => void;
}

export default function VideoTile(props: Props) {
  let videoRef: HTMLVideoElement | undefined;

  const [isHovering, setIsHovering] = createSignal(false);
  const [isPlaying, setIsPlaying] = createSignal(false);

  const shouldPlay = () => {
    if (!props.isActive) return false;
    if (!props.inViewport) return false;
    if (props.autoplay) return props.allowAutoplay;
    return isHovering();
  };

  const videoSrc = createMemo<string | undefined>(() => {
    if (!props.isActive || !props.inViewport) return undefined;
    if (props.autoplay && props.allowAutoplay) return convertFileSrc(props.video.path);
    if (!props.autoplay && isHovering()) return convertFileSrc(props.video.path);
    return undefined;
  });

  createEffect(() => {
    const wantsToPlay = shouldPlay();
    const src = videoSrc();
    if (!videoRef) return;

    if (wantsToPlay && src) {
      if (isPlaying()) return;
      const playAttempt = videoRef.play();
      if (playAttempt) {
        playAttempt.then(() => setIsPlaying(true)).catch(() => setIsPlaying(false));
      } else {
        setIsPlaying(true);
      }
    } else if (isPlaying()) {
      setIsPlaying(false);
      videoRef.pause();
      videoRef.currentTime = 0;
    }
  });
  const thumbSrc = createMemo(() =>
    props.video.thumb_path ? convertFileSrc(props.video.thumb_path) : '',
  );

  const aspectRatio = () => props.video.aspect_ratio ?? 16 / 9;
  const height = () => props.width / aspectRatio();

  return (
    <div
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
      <Show when={thumbSrc() && !isPlaying()}>
        <img
          src={thumbSrc()}
          alt=""
          loading="lazy"
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
      </Show>

      <video
        ref={videoRef}
        src={videoSrc()}
        muted
        loop
        playsinline
        preload={props.autoplay ? 'metadata' : 'none'}
        poster={thumbSrc() || undefined}
        style={{
          width: '100%',
          height: '100%',
          'object-fit': 'cover',
          position: 'absolute',
          top: 0,
          left: 0,
          'background-color': '#1a1a1a',
          opacity: isPlaying() ? 1 : 0,
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
