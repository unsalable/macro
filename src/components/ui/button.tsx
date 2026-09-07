import { Slot } from '@radix-ui/react-slot';
import { cva, type VariantProps } from 'class-variance-authority';
import { forwardRef, type ButtonHTMLAttributes } from 'react';

import { cn } from '@/lib/cn';

const buttonVariants = cva(
  'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-[var(--radius-sm)] text-sm font-medium transition-[background-color,border-color,color,box-shadow,transform] duration-150 disabled:pointer-events-none disabled:opacity-45 [&_svg]:size-4 [&_svg]:shrink-0 active:scale-[0.985]',
  {
    variants: {
      variant: {
        primary:
          'bg-[var(--foreground)] text-[var(--background)] shadow-[var(--shadow-soft)] hover:opacity-90',
        accent:
          'bg-accent text-[var(--accent-foreground)] shadow-[var(--shadow-soft)] hover:brightness-105',
        secondary: 'bg-secondary text-secondary-foreground hover:bg-[var(--muted)]',
        outline:
          'border border-[var(--border-strong)] bg-card text-foreground hover:bg-[var(--card-muted)]',
        ghost: 'text-muted-foreground hover:bg-secondary hover:text-foreground',
        danger: 'bg-danger text-white hover:brightness-105',
        dangerGhost: 'text-danger hover:bg-[var(--danger-soft)]',
      },
      size: {
        sm: 'h-8 px-3 text-[13px]',
        md: 'h-9 px-4',
        lg: 'h-11 px-6 text-[15px]',
        icon: 'size-9',
        iconSm: 'size-8',
      },
    },
    defaultVariants: { variant: 'secondary', size: 'md' },
  },
);

export interface ButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, type = 'button', ...props }, ref) => {
    const Component = asChild ? Slot : 'button';
    return (
      <Component
        ref={ref}
        type={asChild ? undefined : type}
        className={cn(buttonVariants({ variant, size }), className)}
        {...props}
      />
    );
  },
);
Button.displayName = 'Button';

export { buttonVariants };
