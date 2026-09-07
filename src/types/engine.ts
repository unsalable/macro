export type EngineState = 'idle' | 'running' | 'paused' | 'stopping';

export interface EngineStatus {
  state: EngineState;
  macroId: string | null;
  /** Completed loop iterations of the current run. */
  iterations: number;
  /** Index of the action currently executing, -1 when idle. */
  actionIndex: number;
  elapsedMs: number;
  /** Clicks per second measured from the last second of real injections. */
  actualCps: number;
}

export const IDLE_STATUS: EngineStatus = {
  state: 'idle',
  macroId: null,
  iterations: 0,
  actionIndex: -1,
  elapsedMs: 0,
  actualCps: 0,
};

export type RecordedEvent =
  | { at: number; kind: 'key'; action: 'down' | 'up'; code: string }
  | { at: number; kind: 'mouse_button'; action: 'down' | 'up'; button: string }
  | { at: number; kind: 'mouse_move'; x: number; y: number }
  | { at: number; kind: 'mouse_scroll'; direction: 'up' | 'down'; amount: number };

export interface HistoryEntry {
  id: string;
  at: string;
  macroId: string;
  macroName: string;
  event: 'started' | 'stopped' | 'completed' | 'failed';
  iterations: number;
  durationMs: number;
  detail: string | null;
}
