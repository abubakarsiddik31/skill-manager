import { useEffect, type ReactNode } from "react";

interface ModalShellProps {
  /** extra class on the .modal element, e.g. "create-modal" */
  className?: string;
  onClose: () => void;
  children: ReactNode;
}

/** The chrome every modal shares: Escape closes, clicking the overlay
 *  closes, clicks inside the dialog do not. */
export function ModalShell({ className, onClose, children }: ModalShellProps) {
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div
        className={className ? `modal ${className}` : "modal"}
        onClick={(e) => e.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}
