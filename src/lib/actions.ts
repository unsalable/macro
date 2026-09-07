import {
  ArrowRightLeft,
  Clock,
  Keyboard,
  MousePointerClick,
  Repeat,
  ScrollText,
  Type,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

import { formatKeyCode } from '@/lib/hotkey';
import type { ActionType, MacroAction } from '@/types';

export const ACTION_ICONS: Record<ActionType, LucideIcon> = {
  mouse_click: MousePointerClick,
  mouse_move: ArrowRightLeft,
  mouse_scroll: ScrollText,
  key: Keyboard,
  text: Type,
  delay: Clock,
  repeat: Repeat,
};

let counter = 0;

/** Ids only need to be unique inside one macro; Rust never generates them. */
export function newActionId(): string {
  counter += 1;
  return `act_${Date.now().toString(36)}${counter.toString(36)}`;
}

export function createAction(type: ActionType): MacroAction {
  const base = { id: newActionId(), delayBeforeMs: 0, delayAfterMs: 0 };

  switch (type) {
    case 'mouse_click':
      return {
        ...base,
        type,
        button: 'left',
        mode: 'single',
        count: 1,
        intervalMs: 100,
        durationMs: 0,
      };
    case 'mouse_move':
      return { ...base, type, mode: 'absolute', x: 0, y: 0, durationMs: 0, curve: 'smooth' };
    case 'mouse_scroll':
      return { ...base, type, direction: 'down', amount: 3, intervalMs: 30 };
    case 'key':
      return { ...base, type, action: 'press', code: 'KeyA', modifiers: [], durationMs: 0 };
    case 'text':
      return { ...base, type, value: '', perCharDelayMs: 10 };
    case 'delay':
      return { ...base, type, durationMs: 500 };
    case 'repeat':
      return { ...base, type, times: 2, actions: [] };
  }
}

/** Translator shape, kept local so this module does not depend on i18next. */
type Translate = (key: string) => string;

/** One-line description shown on the collapsed card (§8). */
export function describeAction(action: MacroAction, t: Translate): string {
  switch (action.type) {
    case 'mouse_click': {
      const mode = action.mode === 'single' ? '' : ` · ${t(`mouse.${action.mode}`)}`;
      const repeat = action.count > 1 ? ` × ${action.count}` : '';
      return `${t(`mouse.${action.button}`)}${mode}${repeat}`;
    }
    case 'mouse_move':
      return `${t(`action.${action.mode}`)} · ${action.x}, ${action.y}`;
    case 'mouse_scroll':
      return `${t(`action.${action.direction}`)} × ${action.amount}`;
    case 'key': {
      const modifiers = action.modifiers.map((item) => item.toUpperCase()).join('+');
      const key = formatKeyCode(action.code);
      return modifiers ? `${modifiers}+${key}` : key;
    }
    case 'text':
      return action.value.length > 32 ? `${action.value.slice(0, 32)}…` : action.value || '—';
    case 'delay':
      return `${action.durationMs} ms`;
    case 'repeat':
      return `${action.times} × ${action.actions.length}`;
  }
}

/**
 * Best-effort estimate of how long one pass takes, used by the editor's
 * preview strip (§59). Infinite constructs are not represented here.
 */
export function estimateDurationMs(actions: MacroAction[]): number {
  return actions.reduce((total, action) => total + actionDurationMs(action), 0);
}

function actionDurationMs(action: MacroAction): number {
  const padding = action.delayBeforeMs + action.delayAfterMs;

  switch (action.type) {
    case 'mouse_click':
      return padding + action.intervalMs * Math.max(0, action.count - 1) + action.durationMs * action.count;
    case 'mouse_move':
      return padding + action.durationMs;
    case 'mouse_scroll':
      return padding + action.intervalMs * Math.max(0, action.amount - 1);
    case 'key':
      return padding + action.durationMs;
    case 'text':
      return padding + action.perCharDelayMs * action.value.length;
    case 'delay':
      return padding + action.durationMs;
    case 'repeat':
      return padding + estimateDurationMs(action.actions) * action.times;
  }
}
