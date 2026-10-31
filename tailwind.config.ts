import type { Config } from 'tailwindcss';

// Every palette entry resolves through a CSS custom property, so classes like
// `bg-gray-900`, `text-capstone-white` and `bg-capstone-sky` repaint when
// `[data-theme="light"]` is active (see src/index.css) without a single class
// changing. Names keep their dark-theme semantics in both modes: the light
// theme supplies inverted values under the same names, so `--capstone-black`
// means "page background" and `--capstone-white` means "primary foreground"
// rather than literal colours.
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        // The vars hold `R G B` triplets (no rgb(), no commas) so Tailwind's
        // <alpha-value> placeholder can compose opacity modifiers:
        // `bg-capstone-sky/10` → `rgb(var(--capstone-sky) / 0.1)`. Wrapping the
        // value in rgb() in the stylesheet would break every `/NN` class.
        capstone: {
          black: 'rgb(var(--capstone-black) / <alpha-value>)',
          sky: 'rgb(var(--capstone-sky) / <alpha-value>)',
          white: 'rgb(var(--capstone-white) / <alpha-value>)',
        },
        gray: {
          900: 'rgb(var(--gray-900) / <alpha-value>)',
          800: 'rgb(var(--gray-800) / <alpha-value>)',
          700: 'rgb(var(--gray-700) / <alpha-value>)',
          600: 'rgb(var(--gray-600) / <alpha-value>)',
          500: 'rgb(var(--gray-500) / <alpha-value>)',
          400: 'rgb(var(--gray-400) / <alpha-value>)',
          300: 'rgb(var(--gray-300) / <alpha-value>)',
          200: 'rgb(var(--gray-200) / <alpha-value>)',
          100: 'rgb(var(--gray-100) / <alpha-value>)',
        },
      },
      fontFamily: {
        sans: ['Syne', 'system-ui', 'sans-serif'],
        mono: ['"DM Mono"', 'ui-monospace', 'monospace'],
      },
    },
  },
  plugins: [],
} satisfies Config;
