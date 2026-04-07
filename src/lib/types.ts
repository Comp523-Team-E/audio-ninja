export interface FileMetadata {
  fileName: string;
  filePath: string;
  durationMs: number;
  sampleRate: number;
  channels: number;
}

export interface PlaybackPosition {
  positionMs: number;
  isPlaying: boolean;
  durationMs: number;
}

export type MarkerKind = 'start' | 'end' | 'startEnd';

export interface Marker {
  id: string;
  position: number;
  kind: MarkerKind;
}

export interface Segment {
  startMs: number;
  endMs: number;
  title: string;
}
