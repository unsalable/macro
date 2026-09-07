import * as SliderPrimitive from '@radix-ui/react-slider';
import { forwardRef } from 'react';

import { cn } from '@/lib/cn';

export const Slider = forwardRef<
  React.ComponentRef<typeof SliderPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof SliderPrimitive.Root>
>(({ className, ...props }, ref) => (
  <SliderPrimitive.Root
    ref={ref}
    className={cn('relative flex w-full touch-none select-none items-center', className)}
    {...props}
  >
    <SliderPrimitive.Track className="relative h-1.5 w-full grow overflow-hidden rounded-full bg-[var(--border)]">
      <SliderPrimitive.Range className="absolute h-full bg-accent" />
    </SliderPrimitive.Track>
    <SliderPrimitive.Thumb
      className={cn(
        'block size-4 rounded-full border-2 border-accent bg-card shadow-[var(--shadow-soft)] transition-transform',
        'hover:scale-110 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50',
      )}
    />
  </SliderPrimitive.Root>
));
Slider.displayName = 'Slider';
