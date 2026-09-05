// Reused from BSLT main/client/react/src/utils/createFormHandler.ts.
// onStart is inside try so a callback failure cannot leave the UI busy.
export interface FormHandlerOptions {
  onStart?: () => void; onSuccess?: () => void; onError?: (error: Error) => void; onFinally?: () => void;
}
export function createFormHandler(setIsLoading: (value: boolean) => void, setError: (value: string | null) => void, defaults?: FormHandlerOptions) {
  return function wrapHandler<T, R>(handler: (data: T) => Promise<R>, options?: FormHandlerOptions): (data: T) => Promise<R> {
    const callbacks = { ...defaults, ...options };
    return async (data: T): Promise<R> => {
      setIsLoading(true); setError(null);
      try { callbacks.onStart?.(); const result = await handler(data); callbacks.onSuccess?.(); return result; }
      catch (reason) { const error = reason instanceof Error ? reason : new Error('An error occurred'); setError(error.message); callbacks.onError?.(error); throw reason; }
      finally { setIsLoading(false); callbacks.onFinally?.(); }
    };
  };
}
