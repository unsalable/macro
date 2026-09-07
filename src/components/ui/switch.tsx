import * as SwitchPrimitive from '@radix-ui/react-switch';
import { forwardRef } from 'react';

import { cn } from '@/lib/cn';

export const Switch = forwardRef<
  React.ComponentRef<typeof SwitchPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof SwitchPrimitive.Root>
>(({ className, ...props }, ref) => (
  <SwitchPrimitive.Root
    ref={ref}
    className={cn(
      'peer inline-flex h-[22px] w-[38px] shrink-0 cursor-pointer items-center rounded-full border border-transparent transition-colors duration-200',
      'data-[state=checked]:bg-accent data-[state=unchecked]:bg-[var(--border-strong)]',
      'disabled:cursor-not-allowed disabled:opacity-50',
      className,
    )}
    {...props}
  >
    <SwitchPrimitive.Thumb
      className={cn(
        'pointer-events-none block size-[18px] rounded-full bg-white shadow-sm transition-transform duration-200',
        'data-[state=checked]:translate-x-[18px] data-[state=unchecked]:translate-x-[2px]',
      )}
    />
  </SwitchPrimitive.Root>
));
Switch.displayName = 'Switch';

interface SwitchRowProps {
  label: string;
  description?: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  disabled?: boolean;
}

/** The settings-row shape used across the app: text left, switch right (§25). */
export function SwitchRow({
  label,
  description,
  checked,
  onCheckedChange,
  disabled,
}: SwitchRowProps) {
  return (
    <label className="flex cursor-pointer items-center justify-between gap-6 py-3">
      <span className="flex flex-col gap-0.5">
        <span className="text-[13px] font-medium text-foreground">{label}</span>
        {description ? (
          <span className="text-xs leading-relaxed text-muted-foreground">{description}</span>
        ) : null}
      </span>
      <Switch checked={checked} onCheckedChange={onCheckedChange} disabled={disabled ?? false} />
    </label>
  );
}
