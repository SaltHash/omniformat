import { AlertCircle, CheckCircle2, Info, X } from "lucide-react";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import type { Toast } from "../lib/types";

interface ToastInput {
  title: string;
  description?: string;
  tone?: Toast["tone"];
  actions?: Toast["actions"];
}

interface ToastApi {
  pushToast: (toast: ToastInput) => void;
}

const ToastContext = createContext<ToastApi | null>(null);

export function useToast(): ToastApi {
  const context = useContext(ToastContext);
  if (!context) throw new Error("useToast must be used inside ToastProvider");
  return context;
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const timers = useRef(new Map<string, number>());

  const remove = useCallback((id: string) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
    const timer = timers.current.get(id);
    if (timer) window.clearTimeout(timer);
    timers.current.delete(id);
  }, []);

  const pushToast = useCallback(
    (input: ToastInput) => {
      const id =
        typeof crypto !== "undefined" && "randomUUID" in crypto
          ? crypto.randomUUID()
          : `${Date.now()}-${Math.random()}`;
      const toast: Toast = {
        id,
        title: input.title,
        description: input.description,
        tone: input.tone ?? "neutral",
        actions: input.actions,
      };
      setToasts((current) => [...current.slice(-2), toast]);
      const timer = window.setTimeout(
        () => remove(id),
        toast.tone === "error" ? 9000 : 6200,
      );
      timers.current.set(id, timer);
    },
    [remove],
  );

  useEffect(() => {
    const active = timers.current;
    return () => {
      active.forEach((timer) => window.clearTimeout(timer));
      active.clear();
    };
  }, []);

  const api = useMemo(() => ({ pushToast }), [pushToast]);

  return (
    <ToastContext.Provider value={api}>
      {children}
      <div className="toast-stack" aria-live="polite">
        {toasts.map((toast) => {
          const Icon =
            toast.tone === "success"
              ? CheckCircle2
              : toast.tone === "error"
                ? AlertCircle
                : Info;
          return (
            <div key={toast.id} className={`toast toast-${toast.tone}`} role="status">
              <div className="toast-icon">
                <Icon size={16} strokeWidth={1.9} />
              </div>
              <div className="toast-body">
                <div className="toast-title">{toast.title}</div>
                {toast.description ? (
                  <div className="toast-description">{toast.description}</div>
                ) : null}
                {toast.actions?.length ? (
                  <div className="toast-actions">
                    {toast.actions.map((action) => (
                      <button
                        key={action.label}
                        type="button"
                        className="toast-action"
                        onClick={() => {
                          action.onClick();
                          remove(toast.id);
                        }}
                      >
                        {action.label}
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>
              <button
                type="button"
                className="toast-close"
                aria-label="Dismiss"
                onClick={() => remove(toast.id)}
              >
                <X size={13} />
              </button>
            </div>
          );
        })}
      </div>
    </ToastContext.Provider>
  );
}
