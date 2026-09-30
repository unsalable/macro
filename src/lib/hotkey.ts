import type { Hotkey, Modifier } from '@/types';

const MODIFIER_LABELS: Record<Modifier, string> = {
  ctrl: 'Ctrl',
  shift: 'Shift',
  alt: 'Alt',
  win: 'Win',
};

/** Strips the `Key`/`Digit` prefixes so `KeyG` reads as `G` (§14). */
export function formatKeyCode(code: string): string {
  if (code.startsWith('Key')) return code.slice(3);
  if (code.startsWith('Digit')) return code.slice(5);
  if (code.startsWith('Numpad')) return `Num ${code.slice(6)}`;
  if (code.startsWith('Arrow')) return code.slice(5);
  switch (code) {
    // Mouse buttons a hotkey may be bound to (§18).
    case 'MouseMiddle':
      return 'Mouse 3';
    case 'Mouse4':
      return 'Mouse 4';
    case 'Mouse5':
      return 'Mouse 5';
    case 'Escape':
      return 'Esc';
    case 'Backquote':
      return '`';
    case 'Minus':
      return '-';
    case 'Equal':
      return '=';
    case 'BracketLeft':
      return '[';
    case 'BracketRight':
      return ']';
    case 'Backslash':
      return '\\';
    case 'Semicolon':
      return ';';
    case 'Quote':
      return "'";
    case 'Comma':
      return ',';
    case 'Period':
      return '.';
    case 'Slash':
      return '/';
    case 'ControlLeft':
    case 'ControlRight':
      return 'Ctrl';
    case 'ShiftLeft':
    case 'ShiftRight':
      return 'Shift';
    case 'AltLeft':
    case 'AltRight':
      return 'Alt';
    case 'MetaLeft':
    case 'MetaRight':
      return 'Win';
    default:
      return code;
  }
}

const MODIFIER_ORDER: Modifier[] = ['ctrl', 'shift', 'alt', 'win'];

/** Canonical display order, so the same chord always reads the same way. */
export function formatHotkey(hotkey: Hotkey): string {
  const parts = MODIFIER_ORDER.filter((modifier) => hotkey.modifiers.includes(modifier)).map(
    (modifier) => MODIFIER_LABELS[modifier],
  );
  parts.push(formatKeyCode(hotkey.code));
  return parts.join('+');
}

export function hotkeysEqual(a: Hotkey | null, b: Hotkey | null): boolean {
  if (!a || !b) return a === b;
  if (a.code !== b.code) return false;
  if (a.modifiers.length !== b.modifiers.length) return false;
  return a.modifiers.every((modifier) => b.modifiers.includes(modifier));
}
