import type { MarkerKind } from './types';

export function formatMs(ms: number): string {
  const h   = Math.floor(ms / 3_600_000);
  const m   = Math.floor((ms % 3_600_000) / 60_000);
  const s   = Math.floor((ms % 60_000) / 1000);
  const mil = Math.floor(ms % 1000);
  return (
    String(h).padStart(2, '0') + ':' +
    String(m).padStart(2, '0') + ':' +
    String(s).padStart(2, '0') + '.' +
    String(mil).padStart(3, '0')
  );
}

export function kindLabel(kind: MarkerKind): string {
  if (kind === 'start') return 'Start';
  if (kind === 'end')   return 'End';
  return 'Start+End';
}

export const SPEEDS = [0.5, 0.75, 1, 1.5, 2];
