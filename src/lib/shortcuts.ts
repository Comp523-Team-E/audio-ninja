export const ACTION_NAMES = [
  'togglePlay',
  'exportCsv',
  'nudgeLeft',
  'nudgeRight',
  'addStartMarker',
  'addEndMarker',
  'addStartEndMarker',
  'splitStartEndMarker',
  'deleteMarker',
  'stepForward',
  'stepBackward',
  'seekToEnd',
  'seekToStart',
  'toggleFollowPlayhead',
  'seekToPrevMarker',
  'seekToNextMarker',
  'setSpeed1',
  'setSpeed2',
  'setSpeed3',
  'setSpeed4',
  'setSpeed5',
  'zoomOut',
  'zoomIn',
  'cancelEdit',
  'confirmEdit',
] as const;

export type ActionName = (typeof ACTION_NAMES)[number];

export interface ShortcutKey {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
}

export type ShortcutConfig = Record<ActionName, ShortcutKey[]>;

export const DEFAULT_SHORTCUTS: ShortcutConfig = {
  togglePlay:          [{ key: ' ' }],
  exportCsv:           [{ key: 'e', ctrl: true }],
  nudgeLeft:           [{ key: '[' }],
  nudgeRight:          [{ key: ']' }],
  addStartMarker:      [{ key: 's' }],
  addEndMarker:        [{ key: 'e' }],
  addStartEndMarker:   [{ key: 'b' }],
  splitStartEndMarker: [{ key: 'x' }],
  deleteMarker:        [{ key: 'Delete' }, { key: 'Backspace' }],
  stepForward:         [{ key: 'ArrowRight' }],
  stepBackward:        [{ key: 'ArrowLeft' }],
  seekToEnd:           [{ key: 'ArrowUp' }],
  seekToStart:         [{ key: 'ArrowDown' }],
  toggleFollowPlayhead:[{ key: 'l' }],
  seekToPrevMarker:    [{ key: 'd' }],
  seekToNextMarker:    [{ key: 'f' }],
  setSpeed1:           [{ key: '1' }],
  setSpeed2:           [{ key: '2' }],
  setSpeed3:           [{ key: '3' }],
  setSpeed4:           [{ key: '4' }],
  setSpeed5:           [{ key: '5' }],
  zoomOut:             [{ key: '-' }],
  zoomIn:              [{ key: '=' }, { key: '+' }],
  cancelEdit:          [{ key: 'Escape' }],
  confirmEdit:         [{ key: 'Enter' }],
};

export function matchesShortcut(e: KeyboardEvent, keys: ShortcutKey[]): boolean {
  const ctrlOrMeta = e.ctrlKey || e.metaKey;
  return keys.some(k => {
    if (!!k.ctrl !== ctrlOrMeta) return false;
    if (!!k.shift !== e.shiftKey) return false;
    if (!!k.alt !== e.altKey) return false;
    return e.key.toLowerCase() === k.key.toLowerCase();
  });
}

const KEY_DISPLAY: Record<string, string> = {
  ' ':          'Space',
  'arrowleft':  '←',
  'arrowright': '→',
  'arrowup':    '↑',
  'arrowdown':  '↓',
  'escape':     'Esc',
  'delete':     'Del',
  'backspace':  'Bksp',
  'enter':      'Enter',
};

export function formatShortcutKey(k: ShortcutKey): string {
  const parts: string[] = [];
  if (k.ctrl)  parts.push('Ctrl');
  if (k.shift) parts.push('Shift');
  if (k.alt)   parts.push('Alt');
  const display = KEY_DISPLAY[k.key.toLowerCase()] ?? k.key.toUpperCase();
  parts.push(display);
  return parts.join('+');
}
