import type { Marker } from './types';

export function validationProblemMarkerIds(markers: Marker[], error: string | null): Set<string> {
  if (!error) return new Set();

  const sorted = [...markers].sort((a, b) => a.position - b.position);
  const message = error.toLowerCase();

  if (message.includes('no preceding start')) {
    let waitingForEnd = false;
    for (const marker of sorted) {
      if (marker.kind === 'start') waitingForEnd = true;
      else if (marker.kind === 'end') {
        if (!waitingForEnd) return new Set([marker.id]);
        waitingForEnd = false;
      }
    }
  }

  if (message.includes('two consecutive start')) {
    let pendingStart: Marker | null = null;
    for (const marker of sorted) {
      if (marker.kind === 'start') {
        if (pendingStart) return new Set([pendingStart.id, marker.id]);
        pendingStart = marker;
      } else if (marker.kind === 'end') {
        pendingStart = null;
      } else if (marker.kind === 'startEnd') {
        pendingStart = pendingStart ? marker : null;
      }
    }
  }

  if (message.includes('unmatched start')) {
    const position = error.match(/position\s+(\d+)\s+ms/i)?.[1];
    const unmatched = position
      ? sorted.find(marker => (marker.kind === 'start' || marker.kind === 'startEnd') && marker.position === Number(position))
      : [...sorted].reverse().find(marker => marker.kind === 'start' || marker.kind === 'startEnd');
    if (unmatched) return new Set([unmatched.id]);
  }

  return new Set();
}
