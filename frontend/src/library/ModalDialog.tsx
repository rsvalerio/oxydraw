// Accessible modal shell shared by the library panel and the move picker. Mirrors
// Excalidraw's dialog look (dimmed backdrop centring a panel) while supplying the dialog
// semantics the hand-rolled `lib-backdrop`/`lib-panel` markup lacked: role="dialog" +
// aria-modal + an accessible name, Escape-to-close, a Tab focus trap, and focus moved into
// the dialog on open and restored to the opener on close (A11Y-1/A11Y-3).

import { useCallback, useEffect, useId, useRef, type ReactNode } from "react";

// Elements that can take focus inside the dialog — used both to place focus on open and to
// wrap Tab/Shift+Tab at the first/last element (the focus trap).
const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export default function ModalDialog({
  title,
  onClose,
  className,
  children,
}: {
  /** Heading text; also the dialog's accessible name (via aria-labelledby). */
  title: string;
  onClose: () => void;
  /** Extra class on the panel (e.g. `lib-move`). */
  className?: string;
  children: ReactNode;
}) {
  const panelRef = useRef<HTMLDivElement>(null);
  const titleId = useId();

  // Move focus into the dialog on open and restore it to whatever was focused before (the
  // control that opened the modal) when it unmounts.
  useEffect(() => {
    const opener = document.activeElement as HTMLElement | null;
    const panel = panelRef.current;
    const first = panel?.querySelector<HTMLElement>(FOCUSABLE);
    (first ?? panel)?.focus();
    return () => opener?.focus();
  }, []);

  const onKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      const panel = panelRef.current;
      if (event.key === "Escape") {
        // Stop here so a nested dialog's Escape (the move picker) doesn't also close the
        // panel underneath it.
        event.stopPropagation();
        onClose();
        return;
      }
      if (event.key !== "Tab" || !panel) {
        return;
      }
      // This dialog owns Tab while focus is within it; don't let an ancestor dialog re-trap.
      event.stopPropagation();
      const focusables = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE));
      if (focusables.length === 0) {
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const active = document.activeElement;
      if (event.shiftKey && active === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && active === last) {
        event.preventDefault();
        first.focus();
      }
    },
    [onClose],
  );

  return (
    // Backdrop click dismisses (mouse affordance); the close button + Escape are the
    // keyboard equivalents. stopPropagation keeps a nested picker's backdrop click from
    // also closing the panel behind it.
    <div
      className="lib-backdrop"
      onClick={(e) => {
        e.stopPropagation();
        onClose();
      }}
    >
      <div
        ref={panelRef}
        className={className ? `lib-panel ${className}` : "lib-panel"}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <header className="lib-header">
          <span id={titleId}>{title}</span>
          <button type="button" className="lib-close" aria-label="Close" onClick={onClose}>
            ×
          </button>
        </header>
        {children}
      </div>
    </div>
  );
}
