import { Keyboard, MousePointerClick, MoveRight, ScrollText } from 'lucide-react';
import { motion } from 'motion/react';

import { formatKeyCode } from '@/lib/hotkey';
import { transition } from '@/lib/motion';
import type { RecordedEvent } from '@/types';

interface TimelineProps {
  events: RecordedEvent[];
}

/** Newest at the top: while recording, that is where the eye already is. */
export function Timeline({ events }: TimelineProps) {
  return (
    <ol className="flex flex-col gap-1">
      {events
        .slice()
        .reverse()
        .map((event, index) => (
          <motion.li
            key={`${event.at}-${index}`}
            initial={{ opacity: 0, x: -6 }}
            animate={{ opacity: 1, x: 0 }}
            transition={transition.fast}
            className="flex items-center gap-3 rounded-[var(--radius-sm)] bg-card px-3 py-2"
          >
            <span className="flex size-6 shrink-0 items-center justify-center rounded-[7px] bg-[var(--accent-soft)] text-accent">
              <EventIcon kind={event.kind} />
            </span>
            <span className="min-w-0 flex-1 truncate text-[13px]">{describe(event)}</span>
            <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
              {(event.at / 1000).toFixed(2)}s
            </span>
          </motion.li>
        ))}
    </ol>
  );
}

function EventIcon({ kind }: { kind: RecordedEvent['kind'] }) {
  switch (kind) {
    case 'key':
      return <Keyboard className="size-3.5" />;
    case 'mouse_button':
      return <MousePointerClick className="size-3.5" />;
    case 'mouse_scroll':
      return <ScrollText className="size-3.5" />;
    case 'mouse_move':
      return <MoveRight className="size-3.5" />;
  }
}

function describe(event: RecordedEvent): string {
  switch (event.kind) {
    case 'key':
      return `${formatKeyCode(event.code)} ${event.action}`;
    case 'mouse_button':
      return `${event.button} ${event.action}`;
    case 'mouse_move':
      return `${event.x}, ${event.y}`;
    case 'mouse_scroll':
      return `${event.direction} × ${event.amount}`;
  }
}
