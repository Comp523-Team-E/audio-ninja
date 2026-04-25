import { describe, it, expect } from 'vitest';
import { matchesShortcut, formatShortcutKey } from '$lib/shortcuts';
import type { ShortcutKey } from '$lib/shortcuts';

function keyEvent(key: string, extra: Partial<KeyboardEventInit> = {}): KeyboardEvent {
  return new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...extra });
}

describe('matchesShortcut', () => {
  it('matches a plain key', () => {
    expect(matchesShortcut(keyEvent('s'), [{ key: 's' }])).toBe(true);
  });

  it('does not match a different key', () => {
    expect(matchesShortcut(keyEvent('e'), [{ key: 's' }])).toBe(false);
  });

  it('matches case-insensitively for uppercase event', () => {
    expect(matchesShortcut(keyEvent('S'), [{ key: 's' }])).toBe(true);
  });

  it('matches case-insensitively for uppercase config key', () => {
    expect(matchesShortcut(keyEvent('s'), [{ key: 'S' }])).toBe(true);
  });

  it('matches when ctrl is required and ctrlKey is held', () => {
    expect(matchesShortcut(keyEvent('e', { ctrlKey: true }), [{ key: 'e', ctrl: true }])).toBe(true);
  });

  it('matches ctrl shortcut when metaKey is held (cross-platform)', () => {
    expect(matchesShortcut(keyEvent('e', { metaKey: true }), [{ key: 'e', ctrl: true }])).toBe(true);
  });

  it('does not match ctrl shortcut without modifier', () => {
    expect(matchesShortcut(keyEvent('e'), [{ key: 'e', ctrl: true }])).toBe(false);
  });

  it('does not match plain key when ctrl is held but not required', () => {
    expect(matchesShortcut(keyEvent('e', { ctrlKey: true }), [{ key: 'e' }])).toBe(false);
  });

  it('matches when shift is required and shiftKey is held', () => {
    expect(matchesShortcut(keyEvent('a', { shiftKey: true }), [{ key: 'a', shift: true }])).toBe(true);
  });

  it('does not match shift shortcut without shiftKey', () => {
    expect(matchesShortcut(keyEvent('a'), [{ key: 'a', shift: true }])).toBe(false);
  });

  it('matches alt shortcut when altKey is held', () => {
    expect(matchesShortcut(keyEvent('z', { altKey: true }), [{ key: 'z', alt: true }])).toBe(true);
  });

  it('does not match alt shortcut without altKey', () => {
    expect(matchesShortcut(keyEvent('z'), [{ key: 'z', alt: true }])).toBe(false);
  });

  it('matches space key', () => {
    expect(matchesShortcut(keyEvent(' '), [{ key: ' ' }])).toBe(true);
  });

  it('matches arrow keys exactly', () => {
    expect(matchesShortcut(keyEvent('ArrowRight'), [{ key: 'ArrowRight' }])).toBe(true);
    expect(matchesShortcut(keyEvent('ArrowLeft'),  [{ key: 'ArrowLeft'  }])).toBe(true);
  });

  it('does not match wrong arrow direction', () => {
    expect(matchesShortcut(keyEvent('ArrowRight'), [{ key: 'ArrowLeft' }])).toBe(false);
  });

  it('matches the first binding in a multi-key array', () => {
    const keys: ShortcutKey[] = [{ key: 'Delete' }, { key: 'Backspace' }];
    expect(matchesShortcut(keyEvent('Delete'), keys)).toBe(true);
  });

  it('matches the second binding in a multi-key array', () => {
    const keys: ShortcutKey[] = [{ key: 'Delete' }, { key: 'Backspace' }];
    expect(matchesShortcut(keyEvent('Backspace'), keys)).toBe(true);
  });

  it('does not match a key not in the array', () => {
    const keys: ShortcutKey[] = [{ key: 'Delete' }, { key: 'Backspace' }];
    expect(matchesShortcut(keyEvent('x'), keys)).toBe(false);
  });

  it('returns false for an empty array', () => {
    expect(matchesShortcut(keyEvent('s'), [])).toBe(false);
  });
});

describe('formatShortcutKey', () => {
  it('formats a plain letter', () => {
    expect(formatShortcutKey({ key: 's' })).toBe('S');
  });

  it('formats space as "Space"', () => {
    expect(formatShortcutKey({ key: ' ' })).toBe('Space');
  });

  it('formats arrow keys as symbols', () => {
    expect(formatShortcutKey({ key: 'ArrowLeft'  })).toBe('←');
    expect(formatShortcutKey({ key: 'ArrowRight' })).toBe('→');
    expect(formatShortcutKey({ key: 'ArrowUp'    })).toBe('↑');
    expect(formatShortcutKey({ key: 'ArrowDown'  })).toBe('↓');
  });

  it('formats Escape as "Esc"', () => {
    expect(formatShortcutKey({ key: 'Escape' })).toBe('Esc');
  });

  it('formats Delete as "Del"', () => {
    expect(formatShortcutKey({ key: 'Delete' })).toBe('Del');
  });

  it('prepends Ctrl+ for ctrl modifier', () => {
    expect(formatShortcutKey({ key: 'e', ctrl: true })).toBe('Ctrl+E');
  });

  it('prepends Shift+ for shift modifier', () => {
    expect(formatShortcutKey({ key: 'a', shift: true })).toBe('Shift+A');
  });

  it('prepends Alt+ for alt modifier', () => {
    expect(formatShortcutKey({ key: 'z', alt: true })).toBe('Alt+Z');
  });

  it('stacks multiple modifiers in order', () => {
    expect(formatShortcutKey({ key: 's', ctrl: true, shift: true })).toBe('Ctrl+Shift+S');
  });

  it('formats bracket keys as-is', () => {
    expect(formatShortcutKey({ key: '[' })).toBe('[');
    expect(formatShortcutKey({ key: ']' })).toBe(']');
  });
});
