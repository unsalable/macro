import * as LabelPrimitive from '@radix-ui/react-label';
import { forwardRef, type InputHTMLAttributes, type ReactNode } from 'react';

import { cn } from '@/lib/cn';

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn(
        'h-9 w-full rounded-[var(--radius-sm)] border border-[var(--input)] bg-[var(--card-muted)] px-3 text-sm text-foreground transition-colors',
        'placeholder:text-muted-foreground/70 focus-visible:border-accent focus-visible:outline-none',
        'disabled:cursor-not-allowed disabled:opacity-50',
        className,
      )}
      {...props}
    />
  ),
);
Input.displayName = 'Input';

export const Label = forwardRef<
  React.ComponentRef<typeof LabelPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof LabelPrimitive.Root>
>(({ className, ...props }, ref) => (
  <LabelPrimitive.Root
    ref={ref}
    className={cn('text-[13px] font-medium text-foreground', className)}
    {...props}
  />
));
Label.displayName = 'Label';

interface FieldProps {
  label: string;
  hint?: string | undefined;
  htmlFor?: string | undefined;
  className?: string | undefined;
  children: ReactNode;
}

/** Label + control + optional hint, spaced on the 8px grid (§37). */
export function Field({ label, hint, htmlFor, className, children }: FieldProps) {
  return (
    <div className={cn('flex flex-col gap-1.5', className)}>
      <Label htmlFor={htmlFor}>{label}</Label>
      {children}
      {hint ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
    </div>
  );
}

interface NumberInputProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'onChange' | 'value'> {
  value: number;
  onValueChange: (value: number) => void;
  min?: number;
  max?: number;
  suffix?: string;
}

/** Numeric input that never emits NaN and clamps on commit, not per keystroke. */
export function NumberInput({
  value,
  onValueChange,
  min = 0,
  max = Number.MAX_SAFE_INTEGER,
  suffix,
  className,
  ...props
}: NumberInputProps) {
  return (
    <div className="relative">
      <Input
        type="number"
        inputMode="numeric"
        value={Number.isFinite(value) ? value : 0}
        min={min}
        max={max}
        onChange={(event) => {
          const parsed = Number(event.target.value);
          onValueChange(Number.isFinite(parsed) ? parsed : 0);
        }}
        onBlur={() => onValueChange(Math.min(max, Math.max(min, value)))}
        className={cn(suffix ? 'pr-10' : undefined, className)}
        {...props}
      />
      {suffix ? (
        <span className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">
          {suffix}
        </span>
      ) : null}
    </div>
  );
}
