import { X } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Kbd } from '@/components/ui/misc';
import { cn } from '@/lib/cn';
import { formatHotkey } from '@/lib/hotkey';
import type { Hotkey, Modifier } from '@/types';

interface HotkeyInputProps {
  value: Hotkey | null;
  onChange: (value: Hotkey | null) => void;
  disabled?: boolean;
}

const MODIFIER_CODES = new Set([
  'ShiftLeft',
  'ShiftRight',
  'ControlLeft',
  'ControlRight',
  'AltLeft',
  'AltRight',
  'MetaLeft',
  'MetaRight',
]);

/**
 * Mouse buttons that may stand in for a key. Left and right are missing on
 * purpose: binding them would swallow ordinary clicking — including the click
 * that starts and stops this very capture.
 */
const MOUSE_CODES: Record<number, string> = {
  1: 'MouseMiddle',
  3: 'Mouse4',
  4: 'Mouse5',
};

/**
 * Captures a real key combination. `event.code` is used rather than `key`, so
 * the recorded name matches the Rust side exactly and survives layout changes.
 * Mouse side buttons are captured the same way (§18).
 */
export function HotkeyInput({ value, onChange, disabled = false }: HotkeyInputProps) {
  const { t } = useTranslation();
  const [capturing, setCapturing] = useState(false);
  const button = useRef<HTMLButtonElement>(null);

  const stop = useCallback(() => setCapturing(false), []);

  useEffect(() => {
    if (!capturing) return undefined;

    const modifiersOf = (event: MouseEvent | KeyboardEvent): Modifier[] => {
      const modifiers: Modifier[] = [];
      if (event.ctrlKey) modifiers.push('ctrl');
      if (event.shiftKey) modifiers.push('shift');
      if (event.altKey) modifiers.push('alt');
      if (event.metaKey) modifiers.push('win');
      return modifiers;
    };

    const handler = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.code === 'Escape') {
        stop();
        return;
      }
      // Wait for a real key: a lone modifier is not a shortcut.
      if (MODIFIER_CODES.has(event.code)) return;

      onChange({ code: event.code, modifiers: modifiersOf(event) });
      stop();
    };

    const mouseDown = (event: MouseEvent) => {
      const code = MOUSE_CODES[event.button];
      if (!code) return;
      // The webview treats buttons 3 and 4 as back/forward, and the middle
      // button as autoscroll; swallowing the press keeps the capture from
      // navigating the app away from the screen being edited.
      event.preventDefault();
      event.stopPropagation();
      onChange({ code, modifiers: modifiersOf(event) });
      stop();
    };

    // The release and the synthetic click of the same press would still reach
    // the page after the capture ended, so they are swallowed too.
    const swallow = (event: MouseEvent) => {
      if (!MOUSE_CODES[event.button]) return;
      event.preventDefault();
      event.stopPropagation();
    };

    window.addEventListener('keydown', handler, { capture: true });
    window.addEventListener('mousedown', mouseDown, { capture: true });
    window.addEventListener('mouseup', swallow, { capture: true });
    window.addEventListener('auxclick', swallow, { capture: true });
    return () => {
      window.removeEventListener('keydown', handler, { capture: true });
      window.removeEventListener('mousedown', mouseDown, { capture: true });
      window.removeEventListener('mouseup', swallow, { capture: true });
      window.removeEventListener('auxclick', swallow, { capture: true });
    };
  }, [capturing, onChange, stop]);

  return (
    <div className="flex items-center gap-2">
      <button
        ref={button}
        type="button"
        disabled={disabled}
        onClick={() => setCapturing((current) => !current)}
        onBlur={stop}
        className={cn(
          'flex h-9 min-w-40 items-center gap-1.5 rounded-[var(--radius-sm)] border px-3 text-[13px] transition-colors',
          capturing
            ? 'border-accent bg-[var(--accent-soft)] text-accent'
            : 'border-[var(--input)] bg-[var(--card-muted)] text-foreground hover:border-[var(--border-strong)]',
          disabled && 'cursor-not-allowed opacity-50',
        )}
      >
        {capturing ? (
          <span className="text-accent">{t('hotkey.press')}</span>
        ) : value ? (
          formatHotkey(value)
            .split('+')
            .map((part) => <Kbd key={part}>{part}</Kbd>)
        ) : (
          <span className="text-muted-foreground">{t('common.none')}</span>
        )}
      </button>

      {value && !capturing ? (
        <Button
          variant="ghost"
          size="iconSm"
          onClick={() => onChange(null)}
          aria-label={t('hotkey.clear')}
        >
          <X />
        </Button>
      ) : null}
    </div>
  );
}
