// BSLT useFormState, with an immediate guard for rapid double submits.
import { useCallback, useRef, useState } from 'react';
import { createFormHandler, type FormHandlerOptions } from './createFormHandler';
export function useFormState() {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const busy = useRef(false);
  const clearError = useCallback(() => setError(null), []);
  const wrapHandler = useCallback(<T, R>(handler: (data: T) => Promise<R>, options?: FormHandlerOptions) => async (data: T): Promise<R> => {
    if (busy.current) throw new Error('A request is already in progress');
    busy.current = true;
    try { return await createFormHandler(setIsLoading, setError)(handler, options)(data); }
    finally { busy.current = false; }
  }, []);
  return { isLoading, error, setError, clearError, wrapHandler };
}
