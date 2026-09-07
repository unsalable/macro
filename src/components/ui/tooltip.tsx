import * as TooltipPrimitive from '@radix-ui/react-tooltip';
import { forwardRef, type ReactNode } from 'react';

import { cn } from '@/lib/cn';

export const TooltipProvider = TooltipPrimitive.Provider;
export const TooltipRoot = TooltipPrimitive.Root;
export const TooltipTrigger = TooltipPrimitive.Trigger;

export const TooltipContent = forwardRef<
  React.ComponentRef<typeof TooltipPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof TooltipPrimitive.Content>
>(({ className, sideOffset = 6, ...props }, ref) => (
  <TooltipPrimitive.Portal>
    <TooltipPrimitive.Content
      ref={ref}
      sideOffset={sideOffset}
      className={cn(
        'anim-menu z-50 max-w-64 rounded-[var(--radius-sm)] border border-[var(--border)] bg-[var(--popover)] px-2.5 py-1.5',
        'text-xs leading-relaxed text-foreground shadow-[var(--shadow-lift)]',
        className,
      )}
      {...props}
    />
  </TooltipPrimitive.Portal>
));
TooltipContent.displayName = 'TooltipContent';

interface TooltipProps {
  label: ReactNode;
  side?: 'top' | 'right' | 'bottom' | 'left';
  children: ReactNode;
}

/** Shorthand for the common case: wrap a control, show a short label. */
export function Tooltip({ label, side = 'top', children }: TooltipProps) {
  return (
    <TooltipRoot>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side={side}>{label}</TooltipContent>
    </TooltipRoot>
  );
}
