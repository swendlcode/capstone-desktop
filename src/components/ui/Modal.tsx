import { useEffect } from 'react';
import { createPortal } from 'react-dom';
import type { ReactNode } from 'react';
import { CloseCircle } from './icons';

type ModalWidth = 'sm' | 'md' | 'lg';

const WIDTHS: Record<ModalWidth, string> = {
  sm: 'max-w-sm',
  md: 'max-w-lg',
  lg: 'max-w-2xl',
};

interface ModalProps {
  open: boolean;
  onClose: () => void;
  /** Two or three words. The header has one line and does not wrap well. */
  title?: ReactNode;
  /** Optional second line under the title. One short sentence at most. */
  subtitle?: ReactNode;
  children: ReactNode;
  /** Right-aligned action row pinned under the body. */
  footer?: ReactNode;
  width?: ModalWidth;
  /** For a step the user must finish or explicitly cancel. */
  hideClose?: boolean;
  /** Clicking the backdrop dismisses unless this is set. */
  staticBackdrop?: boolean;
  className?: string;
}

/**
 * Portalled dialog over a dimmed, blurred backdrop.
 *
 * Portalled for the same reason the context menu is: an ancestor with
 * `overflow: hidden` or a transform would otherwise clip it or trap it in the
 * wrong stacking context. Mounted only while `open`, so children remount on
 * each show and a half-filled form does not come back.
 */
export function Modal({
  open,
  onClose,
  title,
  subtitle,
  children,
  footer,
  width = 'md',
  hideClose = false,
  staticBackdrop = false,
  className = '',
}: ModalProps) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [open, onClose]);

  if (!open) return null;

  return createPortal(
    <div
      className="fixed inset-0 z-[10000] flex items-center justify-center bg-black/55 p-6 backdrop-blur-sm"
      onMouseDown={() => {
        if (!staticBackdrop) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        // The backdrop closes on mousedown, so the panel has to swallow its own
        // or a drag that ends outside it would dismiss the dialog.
        onMouseDown={(e) => e.stopPropagation()}
        className={`flex max-h-full w-full ${WIDTHS[width]} flex-col overflow-hidden rounded-xl border border-gray-600 bg-gray-800 shadow-[0_24px_64px_rgba(0,0,0,0.55)] ${className}`}
      >
        {title || !hideClose ? (
          <header className="flex items-start gap-3 border-b border-gray-700 px-5 py-3.5">
            <div className="min-w-0 flex-1">
              {title ? (
                <h2 className="truncate text-base font-semibold text-capstone-white">{title}</h2>
              ) : null}
              {subtitle ? <p className="mt-0.5 text-xs text-gray-400">{subtitle}</p> : null}
            </div>
            {hideClose ? null : (
              <button
                type="button"
                aria-label="Close"
                onClick={onClose}
                className="-mr-1 shrink-0 rounded-md p-1 text-gray-400 transition-colors hover:bg-gray-700 hover:text-capstone-white"
              >
                <CloseCircle size={18} />
              </button>
            )}
          </header>
        ) : null}

        <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4 text-sm text-capstone-white">
          {children}
        </div>

        {footer ? (
          <footer className="flex items-center justify-end gap-2 border-t border-gray-700 px-5 py-3">
            {footer}
          </footer>
        ) : null}
      </div>
    </div>,
    document.body,
  );
}
