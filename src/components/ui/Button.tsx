import { forwardRef } from 'react';
import type { ButtonHTMLAttributes, ReactNode } from 'react';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
type Size = 'sm' | 'md' | 'lg';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  icon?: ReactNode;
}

// The accent hover reads the CSS var rather than a Tailwind shade because the
// accent is user-overridable at runtime (see hooks/useTheme.ts) and a literal
// class would not follow it.
const VARIANTS: Record<Variant, string> = {
  primary: 'bg-capstone-sky text-capstone-black hover:bg-[color:var(--color-accent-hover)]',
  secondary: 'bg-gray-700 text-capstone-white hover:bg-gray-600',
  ghost: 'bg-transparent text-gray-300 hover:bg-gray-700 hover:text-capstone-white',
  danger: 'bg-capstone-sky/70 text-capstone-white hover:bg-capstone-sky/90',
};

const SIZES: Record<Size, string> = {
  sm: 'h-7 px-3 text-xs',
  md: 'h-9 px-4 text-sm',
  lg: 'h-11 px-5 text-base',
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ variant = 'secondary', size = 'md', icon, className = '', children, ...rest }, ref) => (
    <button
      ref={ref}
      className={`inline-flex shrink-0 items-center justify-center gap-2 whitespace-nowrap rounded-md font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 ${VARIANTS[variant]} ${SIZES[size]} ${className}`}
      {...rest}
    >
      {icon}
      {children}
    </button>
  ),
);
Button.displayName = 'Button';
