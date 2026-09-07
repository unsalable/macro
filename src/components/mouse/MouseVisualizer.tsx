import { motion, type TargetAndTransition, type Transition } from 'motion/react';
import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { cn } from '@/lib/cn';
import { transition } from '@/lib/motion';
import type { MouseButton } from '@/types';

interface MouseVisualizerProps {
  selected: MouseButton;
  onSelect: (button: MouseButton) => void;
  /** Buttons that already have a binding get a marker on their chip. */
  bound?: MouseButton[];
  /** True while the engine is actually driving this mouse. */
  active?: boolean;
  /** One click every this many ms; paces the press and ripple animation. */
  periodMs?: number | undefined;
  className?: string;
}

/* -----------------------------------------------------------------------
   Geometry — a modern low-profile symmetrical shell, drawn at roughly the
   proportions of a competitive wireless mouse (about 2:1 long to wide, a
   flat wide nose, the widest point behind the middle, a short tail).

   Everything is drawn against the silhouette clip, so a part only has to
   overshoot the outline and the shell trims it to the right edge. That is
   what keeps the seams looking machined instead of drawn.
------------------------------------------------------------------------ */

/** The outline. Symmetrical about x = 160. */
const SHELL =
  'M160 40 C130 40 110 45 100 58 C89 76 78 112 76 156 C74 206 76 268 86 314 ' +
  'C96 360 124 384 160 384 C196 384 224 360 234 314 C244 268 246 205 244 154 ' +
  'C244 112 231 76 220 58 C210 45 190 40 160 40 Z';

/** Button plate halves. The bottom edge dips toward the middle, the way the
    seam on a real shell follows the finger rest. */
const LEFT_KEY = 'M34 22 L156 22 L156 208 C118 221 72 211 34 190 Z';
const RIGHT_KEY = 'M286 22 L164 22 L164 208 C202 221 248 211 286 190 Z';

/** The split between the two main keys, and the seam under them. */
const SPLIT = 'M160 18 L160 209';
const SEAM =
  'M34 190 C72 211 118 221 156 208 L164 208 C202 221 248 211 286 190';

/** A flush side button on the left flank, rounded on the inboard end. */
function sideKey(top: number): string {
  const bottom = top + 38;
  return `M56 ${top} L104 ${top} A8 8 0 0 1 112 ${top + 8} L112 ${bottom - 8} A8 8 0 0 1 104 ${bottom} L56 ${bottom} Z`;
}

const SIDE_TOP: Record<'mouse4' | 'mouse5', number> = { mouse4: 226, mouse5: 270 };

/** Where the click ripple blooms for each button. */
const RIPPLE: Record<MouseButton, { x: number; y: number }> = {
  left: { x: 124, y: 138 },
  right: { x: 196, y: 138 },
  middle: { x: 160, y: 96 },
  mouse4: { x: 98, y: 245 },
  mouse5: { x: 98, y: 289 },
};

const ORDER: MouseButton[] = ['left', 'right', 'middle', 'mouse4', 'mouse5'];

/**
 * Clamped so 100 CPS beats at a watchable rate instead of a blur, and
 * quantised to twentieths so the 10 Hz measurement stream cannot restart the
 * loop before a single press has finished (§56).
 */
function pressSeconds(periodMs: number): number {
  const seconds = Math.min(1.2, Math.max(0.14, periodMs / 1000));
  return Math.round(seconds * 20) / 20;
}

/**
 * The mouse is the control, not a picture of one: every part is a real hit
 * target, and while the engine runs the part being clicked presses itself in
 * time with the measured rate (§10, §56). The names live in the chip row
 * underneath rather than on leader lines, so the shell stays a product shot
 * instead of turning into a labelled diagram.
 */
export function MouseVisualizer({
  selected,
  onSelect,
  bound = [],
  active = false,
  periodMs = 100,
  className,
}: MouseVisualizerProps) {
  const { t } = useTranslation();
  const [hovered, setHovered] = useState<MouseButton | null>(null);
  const uid = useId().replace(/:/g, '');
  const beat = pressSeconds(periodMs);

  /** Props shared by every clickable region. */
  const region = (button: MouseButton) => ({
    role: 'button' as const,
    tabIndex: 0,
    'aria-label': t(`mouse.${button}`),
    'aria-pressed': selected === button,
    onClick: () => onSelect(button),
    onMouseEnter: () => setHovered(button),
    onMouseLeave: () => setHovered((current) => (current === button ? null : current)),
    onKeyDown: (event: React.KeyboardEvent) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        onSelect(button);
      }
    },
    className: 'cursor-pointer outline-none',
  });

  /** The whole part sinks on press; springs when idle, beats when running. */
  const press = (
    button: MouseButton,
  ): { animate: TargetAndTransition; transition: Transition } => {
    const isSelected = selected === button;
    if (active && isSelected) {
      return {
        animate: { y: [0, 2.5, 0] },
        transition: { duration: beat, repeat: Infinity, times: [0, 0.32, 1], ease: 'easeOut' },
      };
    }
    return {
      animate: { y: isSelected ? 1.2 : 0 },
      transition: transition.spring,
    };
  };

  const fill = (button: MouseButton, base: string) =>
    selected === button ? `url(#${uid}-accent)` : base;

  const wash = (button: MouseButton) =>
    hovered === button && selected !== button ? 0.14 : 0;

  return (
    <div className={cn('flex w-full flex-col items-center gap-5', className)}>
      <svg
        viewBox="0 0 320 428"
        className="h-auto w-full max-w-[248px]"
        role="group"
        aria-label={t('mouse.button')}
      >
        <defs>
          {/* One light source, high and slightly left. Matte, not glossy:
              the gradients are shallow and there is no specular hotspot. */}
          <linearGradient id={`${uid}-shell`} x1="0.2" y1="0" x2="0.8" y2="1">
            <stop offset="0%" stopColor="var(--mouse-shell-top)" />
            <stop offset="100%" stopColor="var(--mouse-shell-bottom)" />
          </linearGradient>
          <linearGradient id={`${uid}-key`} x1="0.2" y1="0" x2="0.85" y2="1">
            <stop offset="0%" stopColor="var(--mouse-key-top)" />
            <stop offset="100%" stopColor="var(--mouse-key-bottom)" />
          </linearGradient>
          <linearGradient id={`${uid}-accent`} x1="0.15" y1="0" x2="0.9" y2="1">
            <stop offset="0%" stopColor="var(--accent)" stopOpacity="0.95" />
            <stop offset="100%" stopColor="var(--accent)" stopOpacity="0.7" />
          </linearGradient>
          <linearGradient id={`${uid}-wheel`} x1="0" y1="0" x2="1" y2="0">
            <stop offset="0%" stopColor="var(--mouse-wheel-bottom)" />
            <stop offset="42%" stopColor="var(--mouse-wheel-top)" />
            <stop offset="100%" stopColor="var(--mouse-wheel-bottom)" />
          </linearGradient>
          {/* Edge darkening: the shell is lifted off the page by the way the
              flanks fall away, not by a drop shadow around the outline. */}
          <radialGradient id={`${uid}-vignette`} cx="0.5" cy="0.42" r="0.62">
            <stop offset="62%" stopColor="var(--mouse-shell-base)" stopOpacity="0" />
            <stop offset="100%" stopColor="var(--mouse-shell-base)" stopOpacity="0.42" />
          </radialGradient>
          <linearGradient id={`${uid}-rim`} x1="0.15" y1="0" x2="0.7" y2="1">
            <stop offset="0%" stopColor="var(--mouse-gloss)" stopOpacity="0.55" />
            <stop offset="55%" stopColor="var(--mouse-gloss)" stopOpacity="0" />
          </linearGradient>
          <filter id={`${uid}-blur`} x="-40%" y="-40%" width="180%" height="180%">
            <feGaussianBlur stdDeviation="9" />
          </filter>
          <clipPath id={`${uid}-clip`}>
            <path d={SHELL} />
          </clipPath>
        </defs>

        {/* Contact shadow: tight under the tail, where the shell actually
            touches the desk. */}
        <ellipse
          cx={160}
          cy={396}
          rx={76}
          ry={12}
          fill="var(--mouse-shadow)"
          filter={`url(#${uid}-blur)`}
        />

        {/* A sliver of the flank, so the top view still reads as a body. */}
        <path d={SHELL} transform="translate(0 6)" fill="var(--mouse-shell-base)" opacity={0.5} />

        <path d={SHELL} fill={`url(#${uid}-shell)`} />

        <g clipPath={`url(#${uid}-clip)`}>
          {/* -------------------------------------------------- side keys */}
          {(['mouse4', 'mouse5'] as const).map((button) => (
            <g key={button} {...region(button)}>
              {/* The recess the key sits in, then the key itself: two
                  tones are what make it read as inset rather than painted. */}
              <path
                d={sideKey(SIDE_TOP[button] - 2)}
                transform="translate(3 0)"
                fill="var(--mouse-groove)"
                opacity={0.85}
              />
              <motion.g {...press(button)}>
                <path
                  d={sideKey(SIDE_TOP[button])}
                  fill={fill(button, `url(#${uid}-key)`)}
                  className="transition-[fill] duration-[var(--fm-normal)]"
                />
                <path
                  d={sideKey(SIDE_TOP[button])}
                  fill="var(--accent)"
                  opacity={wash(button)}
                  className="pointer-events-none transition-opacity duration-[var(--fm-fast)]"
                />
              </motion.g>
            </g>
          ))}

          {/* ------------------------------------------------- main plate */}
          <g {...region('left')}>
            <motion.g {...press('left')}>
              <path
                d={LEFT_KEY}
                fill={fill('left', `url(#${uid}-key)`)}
                className="transition-[fill] duration-[var(--fm-normal)]"
              />
              <path
                d={LEFT_KEY}
                fill="var(--accent)"
                opacity={wash('left')}
                className="pointer-events-none transition-opacity duration-[var(--fm-fast)]"
              />
            </motion.g>
          </g>

          <g {...region('right')}>
            <motion.g {...press('right')}>
              <path
                d={RIGHT_KEY}
                fill={fill('right', `url(#${uid}-key)`)}
                className="transition-[fill] duration-[var(--fm-normal)]"
              />
              <path
                d={RIGHT_KEY}
                fill="var(--accent)"
                opacity={wash('right')}
                className="pointer-events-none transition-opacity duration-[var(--fm-fast)]"
              />
            </motion.g>
          </g>

          {/* The two seams that make the plate read as machined: the split
              between the keys, and the line where the plate meets the palm.
              Drawn over both halves so they stay one continuous cut. */}
          <path
            d={SPLIT}
            fill="none"
            stroke="var(--mouse-groove)"
            strokeWidth={5}
            strokeLinecap="round"
            className="pointer-events-none"
          />
          <path
            d={SEAM}
            fill="none"
            stroke="var(--mouse-groove)"
            strokeWidth={2}
            className="pointer-events-none"
          />

          {/* ----------------------------------------------------- wheel */}
          <g {...region('middle')}>
            {/* Well first, then the wheel sitting down inside it. */}
            <rect x={147} y={56} width={26} height={64} rx={13} fill="var(--mouse-groove)" />
            <motion.g {...press('middle')}>
              <rect
                x={150}
                y={60}
                width={20}
                height={56}
                rx={10}
                fill={fill('middle', `url(#${uid}-wheel)`)}
                className="transition-[fill] duration-[var(--fm-normal)]"
              />
              {[71, 79, 87, 95, 103].map((y) => (
                <line
                  key={y}
                  x1={154}
                  y1={y}
                  x2={166}
                  y2={y}
                  stroke="var(--mouse-groove)"
                  strokeWidth={1.5}
                  strokeLinecap="round"
                  opacity={0.55}
                />
              ))}
              <rect
                x={150}
                y={60}
                width={20}
                height={56}
                rx={10}
                fill="var(--accent)"
                opacity={wash('middle')}
                className="pointer-events-none transition-opacity duration-[var(--fm-fast)]"
              />
            </motion.g>
          </g>

          {/* --------------------------------------------------- surface */}
          {/* Palm mark. It is the one place the shell carries any branding,
              and it lights while the engine is driving the mouse. */}
          <g className="pointer-events-none">
            <circle
              cx={160}
              cy={296}
              r={11}
              fill="none"
              stroke={active ? 'var(--accent)' : 'var(--mouse-groove)'}
              strokeWidth={1.5}
              opacity={active ? 0.9 : 0.5}
              className="transition-[stroke,opacity] duration-[var(--fm-normal)]"
            />
            <circle
              cx={160}
              cy={296}
              r={3.5}
              fill={active ? 'var(--accent)' : 'var(--mouse-groove)'}
              opacity={active ? 0.9 : 0.5}
              className="transition-[fill,opacity] duration-[var(--fm-normal)]"
            />
          </g>

          {/* Every injected click blooms once, in time with the real rate. */}
          {active ? (
            <motion.circle
              key={selected}
              cx={RIPPLE[selected].x}
              cy={RIPPLE[selected].y}
              r={11}
              fill="var(--accent)"
              initial={{ opacity: 0.38, scale: 0.5 }}
              animate={{ opacity: 0, scale: 2.4 }}
              transition={{ duration: beat, repeat: Infinity, ease: 'easeOut' }}
              style={{ transformBox: 'fill-box', transformOrigin: 'center' }}
              className="pointer-events-none"
            />
          ) : null}

          {/* Shading last, over everything, so a selected key falls away at
              the flanks exactly like the shell around it. */}
          <path d={SHELL} fill={`url(#${uid}-vignette)`} className="pointer-events-none" />
        </g>

        {/* Rim light on the lit edge, then a hairline outline to keep the
            silhouette crisp against the card. */}
        <path
          d={SHELL}
          fill="none"
          stroke={`url(#${uid}-rim)`}
          strokeWidth={2}
          className="pointer-events-none"
        />
        <path
          d={SHELL}
          fill="none"
          stroke="var(--mouse-shell-edge)"
          strokeWidth={1.25}
          className="pointer-events-none"
        />
      </svg>

      {/* The names live here, not on the shell. */}
      <div className="flex flex-wrap justify-center gap-1.5">
        {ORDER.map((button) => (
          <button
            key={button}
            type="button"
            onClick={() => onSelect(button)}
            onMouseEnter={() => setHovered(button)}
            onMouseLeave={() => setHovered((current) => (current === button ? null : current))}
            className={cn(
              'flex h-7 items-center gap-1.5 rounded-full border px-2.5 text-[12px] font-medium',
              'transition-colors duration-[var(--fm-fast)]',
              selected === button
                ? 'border-transparent bg-accent text-[var(--accent-foreground)]'
                : 'border-[var(--border)] text-muted-foreground hover:border-[var(--border-strong)] hover:text-foreground',
            )}
          >
            {t(`mouse.${button}`)}
            {bound.includes(button) ? (
              <span
                className={cn(
                  'size-1.5 rounded-full',
                  selected === button ? 'bg-[var(--accent-foreground)]' : 'bg-success',
                )}
              />
            ) : null}
          </button>
        ))}
      </div>

      <p className="text-center text-[12px] text-muted-foreground">{t('mouse.pickHint')}</p>
    </div>
  );
}
