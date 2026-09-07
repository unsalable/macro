/**
 * Mirrors `src-tauri/src/macro/model.rs` one-to-one.
 * Rust serialises with `#[serde(rename_all = "camelCase")]`.
 */

export const SCHEMA_VERSION = 1;

export type MouseButton = 'left' | 'right' | 'middle' | 'mouse4' | 'mouse5';
export const MOUSE_BUTTONS: readonly MouseButton[] = [
  'left',
  'right',
  'middle',
  'mouse4',
  'mouse5',
];

export type ClickMode = 'single' | 'double' | 'hold';
export type Modifier = 'ctrl' | 'shift' | 'alt' | 'win';
export const MODIFIERS: readonly Modifier[] = ['ctrl', 'shift', 'alt', 'win'];

export type MoveMode = 'absolute' | 'relative';
export type MoveCurve = 'linear' | 'smooth';
export type ScrollDirection = 'up' | 'down';
export type KeyPhase = 'press' | 'down' | 'up';

export interface ActionBase {
  id: string;
  delayBeforeMs: number;
  delayAfterMs: number;
}

export type ActionKind =
  | {
      type: 'mouse_click';
      button: MouseButton;
      mode: ClickMode;
      count: number;
      intervalMs: number;
      /** Only meaningful when mode === 'hold'. */
      durationMs: number;
    }
  | {
      type: 'mouse_move';
      mode: MoveMode;
      x: number;
      y: number;
      durationMs: number;
      curve: MoveCurve;
    }
  | { type: 'mouse_scroll'; direction: ScrollDirection; amount: number; intervalMs: number }
  | { type: 'key'; action: KeyPhase; code: string; modifiers: Modifier[]; durationMs: number }
  | { type: 'text'; value: string; perCharDelayMs: number }
  | { type: 'delay'; durationMs: number }
  | { type: 'repeat'; times: number; actions: MacroAction[] };

export type MacroAction = ActionBase & ActionKind;
export type ActionType = ActionKind['type'];

export const ACTION_TYPES: readonly ActionType[] = [
  'mouse_click',
  'mouse_move',
  'mouse_scroll',
  'key',
  'text',
  'delay',
  'repeat',
];

export interface Hotkey {
  code: string;
  modifiers: Modifier[];
}

export type Activation = 'toggle' | 'hold';
export type LoopMode = 'none' | 'infinite' | 'count' | 'duration';

export interface LoopConfig {
  mode: LoopMode;
  count: number;
  durationMs: number;
  /** Gap between two iterations. 0 = as fast as the actions allow. */
  intervalMs: number;
}

export interface Randomization {
  enabled: boolean;
  jitterMs: number;
}

export interface MacroStats {
  runCount: number;
  lastRunAt: string | null;
}

export interface Macro {
  id: string;
  name: string;
  enabled: boolean;
  hotkey: Hotkey | null;
  activation: Activation;
  actions: MacroAction[];
  loop: LoopConfig;
  randomization: Randomization;
  stats: MacroStats;
  createdAt: string;
  updatedAt: string;
}

export type ProfileColor = 'sand' | 'clay' | 'sage' | 'slate' | 'plum' | 'ochre';
export const PROFILE_COLORS: readonly ProfileColor[] = [
  'sand',
  'clay',
  'sage',
  'slate',
  'plum',
  'ochre',
];

export interface Profile {
  id: string;
  name: string;
  color: ProfileColor;
  macros: Macro[];
}

export interface ProfileStore {
  schemaVersion: number;
  activeProfileId: string;
  profiles: Profile[];
}

export interface MacroExport {
  schemaVersion: number;
  kind: 'macro';
  exportedAt: string;
  macro: Macro;
}
