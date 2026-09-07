import { motion } from 'motion/react';

import { cn } from '@/lib/cn';
import { formatKeyCode } from '@/lib/hotkey';
import { transition } from '@/lib/motion';

interface KeyDef {
  code: string;
  width?: number;
  gapBefore?: number;
}

/** US ANSI, tenkeyless. Widths are in key units; 1u is one standard cap. */
const ROWS: KeyDef[][] = [
  [
    { code: 'Escape' },
    { code: 'F1', gapBefore: 0.5 },
    { code: 'F2' },
    { code: 'F3' },
    { code: 'F4' },
    { code: 'F5', gapBefore: 0.25 },
    { code: 'F6' },
    { code: 'F7' },
    { code: 'F8' },
    { code: 'F9', gapBefore: 0.25 },
    { code: 'F10' },
    { code: 'F11' },
    { code: 'F12' },
  ],
  [
    { code: 'Backquote' },
    { code: 'Digit1' },
    { code: 'Digit2' },
    { code: 'Digit3' },
    { code: 'Digit4' },
    { code: 'Digit5' },
    { code: 'Digit6' },
    { code: 'Digit7' },
    { code: 'Digit8' },
    { code: 'Digit9' },
    { code: 'Digit0' },
    { code: 'Minus' },
    { code: 'Equal' },
    { code: 'Backspace', width: 2 },
  ],
  [
    { code: 'Tab', width: 1.5 },
    { code: 'KeyQ' },
    { code: 'KeyW' },
    { code: 'KeyE' },
    { code: 'KeyR' },
    { code: 'KeyT' },
    { code: 'KeyY' },
    { code: 'KeyU' },
    { code: 'KeyI' },
    { code: 'KeyO' },
    { code: 'KeyP' },
    { code: 'BracketLeft' },
    { code: 'BracketRight' },
    { code: 'Backslash', width: 1.5 },
  ],
  [
    { code: 'CapsLock', width: 1.75 },
    { code: 'KeyA' },
    { code: 'KeyS' },
    { code: 'KeyD' },
    { code: 'KeyF' },
    { code: 'KeyG' },
    { code: 'KeyH' },
    { code: 'KeyJ' },
    { code: 'KeyK' },
    { code: 'KeyL' },
    { code: 'Semicolon' },
    { code: 'Quote' },
    { code: 'Enter', width: 2.25 },
  ],
  [
    { code: 'ShiftLeft', width: 2.25 },
    { code: 'KeyZ' },
    { code: 'KeyX' },
    { code: 'KeyC' },
    { code: 'KeyV' },
    { code: 'KeyB' },
    { code: 'KeyN' },
    { code: 'KeyM' },
    { code: 'Comma' },
    { code: 'Period' },
    { code: 'Slash' },
    { code: 'ShiftRight', width: 2.75 },
  ],
  [
    { code: 'ControlLeft', width: 1.25 },
    { code: 'MetaLeft', width: 1.25 },
    { code: 'AltLeft', width: 1.25 },
    { code: 'Space', width: 6.25 },
    { code: 'AltRight', width: 1.25 },
    { code: 'MetaRight', width: 1.25 },
    { code: 'ContextMenu', width: 1.25 },
    { code: 'ControlRight', width: 1.25 },
  ],
];

const ARROWS: KeyDef[][] = [[{ code: 'ArrowUp' }], [{ code: 'ArrowLeft' }, { code: 'ArrowDown' }, { code: 'ArrowRight' }]];

const UNIT = 38;
const GAP = 4;

interface KeyboardVisualizerProps {
  selected: string | null;
  onSelect: (code: string) => void;
  /** Keys already used by a macro get an accent outline. */
  used?: string[];
}

export function KeyboardVisualizer({ selected, onSelect, used = [] }: KeyboardVisualizerProps) {
  return (
    <div className="flex flex-col items-center gap-4">
      <div className="flex flex-col gap-1">
        {ROWS.map((row, rowIndex) => (
          <div key={rowIndex} className="flex gap-1">
            {row.map((key) => (
              <KeyCap
                key={key.code}
                def={key}
                selected={selected === key.code}
                used={used.includes(key.code)}
                onSelect={onSelect}
              />
            ))}
          </div>
        ))}
      </div>

      <div className="flex flex-col items-center gap-1">
        {ARROWS.map((row, rowIndex) => (
          <div key={rowIndex} className="flex gap-1">
            {row.map((key) => (
              <KeyCap
                key={key.code}
                def={key}
                selected={selected === key.code}
                used={used.includes(key.code)}
                onSelect={onSelect}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

interface KeyCapProps {
  def: KeyDef;
  selected: boolean;
  used: boolean;
  onSelect: (code: string) => void;
}

function KeyCap({ def, selected, used, onSelect }: KeyCapProps) {
  const width = (def.width ?? 1) * UNIT + ((def.width ?? 1) - 1) * GAP;

  return (
    <motion.button
      type="button"
      onClick={() => onSelect(def.code)}
      whileTap={{ scale: 0.94 }}
      transition={transition.fast}
      style={{ width, marginLeft: def.gapBefore ? def.gapBefore * UNIT : undefined }}
      className={cn(
        'h-[38px] shrink-0 rounded-[8px] border text-[11px] font-medium transition-colors',
        selected
          ? 'border-accent bg-accent text-[var(--accent-foreground)]'
          : used
            ? 'border-accent bg-[var(--accent-soft)] text-accent'
            : 'border-[var(--border)] bg-card text-muted-foreground hover:border-[var(--border-strong)] hover:text-foreground',
      )}
    >
      <span className="truncate px-1">{formatKeyCode(def.code)}</span>
    </motion.button>
  );
}
