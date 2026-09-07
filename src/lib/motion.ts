import type { Transition, Variants } from 'motion/react';

/**
 * Central motion tokens (§67). No component defines its own curve.
 * Settings -> Performance -> Animation Quality scales these globally.
 */
export const duration = {
  fast: 0.12,
  normal: 0.2,
  slow: 0.3,
} as const;

export const ease = [0.22, 1, 0.36, 1] as const;

export const spring: Transition = {
  type: 'spring',
  stiffness: 380,
  damping: 32,
  mass: 0.8,
};

export const stagger = 0.035;

export const transition = {
  fast: { duration: duration.fast, ease } satisfies Transition,
  normal: { duration: duration.normal, ease } satisfies Transition,
  slow: { duration: duration.slow, ease } satisfies Transition,
  spring,
} as const;

/**
 * Page-level enter/exit used by the router transition. `custom` carries the
 * navigation direction (+1 down the sidebar, -1 back up) so the page always
 * travels the same way the selection does.
 */
export const pageVariants: Variants = {
  initial: (direction: number) => ({ opacity: 0, y: 10 * (direction || 1) }),
  animate: { opacity: 1, y: 0 },
  exit: (direction: number) => ({ opacity: 0, y: -8 * (direction || 1) }),
};

/** List container that staggers its children in. */
export const listVariants: Variants = {
  initial: {},
  animate: { transition: { staggerChildren: stagger } },
};

export const listItemVariants: Variants = {
  initial: { opacity: 0, y: 10 },
  animate: { opacity: 1, y: 0 },
};

export type AnimationQuality = 'low' | 'balanced' | 'high';

/** Multiplier applied to every duration for the given quality level (§46). */
export function speedFactor(quality: AnimationQuality): number {
  switch (quality) {
    case 'low':
      return 0;
    case 'balanced':
      return 0.8;
    case 'high':
      return 1;
  }
}
