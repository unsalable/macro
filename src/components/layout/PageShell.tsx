import { motion } from 'motion/react';
import type { ReactNode } from 'react';

import { cn } from '@/lib/cn';
import { pageVariants, transition } from '@/lib/motion';
import { useNavDirection } from '@/lib/nav';

interface PageShellProps {
  title: string;
  description?: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}

/** Every page shares this frame: header row, then a scrolling body. */
export function PageShell({ title, description, actions, children, className }: PageShellProps) {
  const direction = useNavDirection();

  return (
    <motion.div
      custom={direction}
      variants={pageVariants}
      initial="initial"
      animate="animate"
      exit="exit"
      transition={transition.normal}
      className="flex h-full flex-col overflow-hidden"
    >
      <div className="flex shrink-0 items-start justify-between gap-6 px-8 pb-5 pt-7">
        <div className="flex flex-col gap-1">
          <h1 className="text-[22px] font-semibold leading-tight tracking-tight">{title}</h1>
          {description ? (
            <p className="text-[13px] text-muted-foreground">{description}</p>
          ) : null}
        </div>
        {actions ? <div className="flex shrink-0 items-center gap-2">{actions}</div> : null}
      </div>

      <div className={cn('scrollbar-thin flex-1 overflow-y-auto px-8 pb-8', className)}>
        {children}
      </div>
    </motion.div>
  );
}
