import * as TabsPrimitive from '@radix-ui/react-tabs';
import { motion } from 'motion/react';
import { createContext, forwardRef, useContext, useId } from 'react';

import { cn } from '@/lib/cn';
import { transition } from '@/lib/motion';

interface TabsContextValue {
  value: string;
  group: string;
}

const TabsContext = createContext<TabsContextValue>({ value: '', group: 'tabs' });

export const TabsContent = TabsPrimitive.Content;

type TabsProps = Omit<React.ComponentPropsWithoutRef<typeof TabsPrimitive.Root>, 'value'> & {
  /** Controlled on purpose: the sliding indicator needs to know what is active. */
  value: string;
};

export function Tabs({ value, children, ...props }: TabsProps) {
  const group = useId();
  return (
    <TabsContext.Provider value={{ value, group }}>
      <TabsPrimitive.Root value={value} {...props}>
        {children}
      </TabsPrimitive.Root>
    </TabsContext.Provider>
  );
}

export const TabsList = forwardRef<
  React.ComponentRef<typeof TabsPrimitive.List>,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.List>
>(({ className, ...props }, ref) => (
  <TabsPrimitive.List
    ref={ref}
    className={cn(
      'inline-flex items-center gap-1 rounded-[var(--radius-sm)] bg-secondary p-1',
      className,
    )}
    {...props}
  />
));
TabsList.displayName = 'TabsList';

export const TabsTrigger = forwardRef<
  React.ComponentRef<typeof TabsPrimitive.Trigger>,
  React.ComponentPropsWithoutRef<typeof TabsPrimitive.Trigger>
>(({ className, children, value, ...props }, ref) => {
  const context = useContext(TabsContext);
  const active = context.value === value;

  return (
    <TabsPrimitive.Trigger
      ref={ref}
      value={value}
      className={cn(
        'relative isolate inline-flex h-7 items-center justify-center rounded-[7px] px-3 text-[13px] font-medium',
        'text-muted-foreground transition-colors hover:text-foreground',
        'data-[state=active]:text-foreground focus-visible:outline-none',
        className,
      )}
      {...props}
    >
      {active ? (
        // One shared layoutId per group, so the pill slides between tabs
        // instead of teleporting (§28).
        <motion.span
          aria-hidden
          layoutId={`tabs-indicator-${context.group}`}
          className="absolute inset-0 rounded-[7px] bg-card shadow-[var(--shadow-soft)]"
          transition={transition.spring}
        />
      ) : null}
      <span className="relative z-10">{children}</span>
    </TabsPrimitive.Trigger>
  );
});
TabsTrigger.displayName = 'TabsTrigger';
