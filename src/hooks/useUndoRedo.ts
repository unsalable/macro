import { useCallback, useRef, useState } from 'react';

interface UndoRedo<T> {
  state: T;
  set: (next: T | ((current: T) => T)) => void;
  /** Replaces the state without adding a history entry (e.g. after a load). */
  reset: (next: T) => void;
  undo: () => void;
  redo: () => void;
  canUndo: boolean;
  canRedo: boolean;
}

const LIMIT = 100;

/**
 * History stack for the macro editor (§58). Consecutive edits within
 * `coalesceMs` collapse into one entry, so dragging a slider does not produce
 * fifty undo steps.
 */
export function useUndoRedo<T>(initial: T, coalesceMs = 400): UndoRedo<T> {
  const [state, setState] = useState<T>(initial);
  const past = useRef<T[]>([]);
  const future = useRef<T[]>([]);
  const lastPush = useRef(0);
  const [version, setVersion] = useState(0);

  const set = useCallback(
    (next: T | ((current: T) => T)) => {
      setState((current) => {
        const resolved = typeof next === 'function' ? (next as (value: T) => T)(current) : next;
        if (Object.is(resolved, current)) return current;

        const now = Date.now();
        if (now - lastPush.current > coalesceMs) {
          past.current = [...past.current, current].slice(-LIMIT);
        }
        lastPush.current = now;
        future.current = [];
        setVersion((value) => value + 1);
        return resolved;
      });
    },
    [coalesceMs],
  );

  const reset = useCallback((next: T) => {
    past.current = [];
    future.current = [];
    lastPush.current = 0;
    setState(next);
    setVersion((value) => value + 1);
  }, []);

  const undo = useCallback(() => {
    setState((current) => {
      const previous = past.current[past.current.length - 1];
      if (previous === undefined) return current;
      past.current = past.current.slice(0, -1);
      future.current = [current, ...future.current].slice(0, LIMIT);
      lastPush.current = 0;
      setVersion((value) => value + 1);
      return previous;
    });
  }, []);

  const redo = useCallback(() => {
    setState((current) => {
      const next = future.current[0];
      if (next === undefined) return current;
      future.current = future.current.slice(1);
      past.current = [...past.current, current].slice(-LIMIT);
      lastPush.current = 0;
      setVersion((value) => value + 1);
      return next;
    });
  }, []);

  // `version` exists so the booleans below re-evaluate on every history change.
  void version;

  return {
    state,
    set,
    reset,
    undo,
    redo,
    canUndo: past.current.length > 0,
    canRedo: future.current.length > 0,
  };
}
