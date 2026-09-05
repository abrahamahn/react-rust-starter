// Thin HTTP boundary replacing BSLT client-engine coupling. Never retries writes.
import { parseSession } from './contracts';
const messages: Record<string, string> = {
  invalid_input: 'Check the fields and try again.', invalid_credentials: 'The email or password is incorrect.',
  unauthenticated: 'Your session ended. Sign in again.', email_unverified: 'Verify your email before using notes.',
  account_unavailable: 'This account cannot be created. Try signing in or resetting your password.',
  invalid_or_expired_token: 'This link is invalid, expired, or already used. Request a new one.',
  conflict: 'This item changed, or the 100-note limit was reached. Reload before trying again.',
  not_found: 'This item is no longer available.', rate_limited: 'Too many attempts. Try again in five minutes.',
  service_unavailable: 'The service is unavailable. Please retry later.', origin_rejected: 'This app origin is not allowed. Check APP_ORIGIN.',
};
export class ApiError extends Error { constructor(public code: string, public status: number) { super(messages[code] ?? 'The request failed. Please try again.'); } }
export async function request(path: string, options: { method?: string; body?: unknown; signal?: AbortSignal } = {}): Promise<unknown> {
  const method = options.method ?? 'GET';
  let response: Response;
  try {
    response = await fetch(`/api${path}`, { method, credentials: 'same-origin', cache: 'no-store', signal: options.signal,
      headers: { ...(method !== 'GET' ? { 'X-Starter-Client': 'web-v1' } : {}), ...(options.body !== undefined ? { 'Content-Type': 'application/json' } : {}) },
      body: options.body === undefined ? undefined : JSON.stringify(options.body) });
  } catch (error) {
    if (options.signal?.aborted) throw error;
    throw new Error('Cannot reach the server. Check your connection; a submitted change may need verification.');
  }
  if (!response.ok) {
    const value = await response.json().catch(() => null);
    const code = typeof value?.error?.code === 'string' ? value.error.code : 'request_failed';
    if (code === 'unauthenticated') window.dispatchEvent(new Event('starter:unauthenticated'));
    throw new ApiError(code, response.status);
  }
  if (response.status === 204 || response.status === 202) return null;
  return response.json();
}
export async function currentSession(signal?: AbortSignal) {
  try { return parseSession(await request('/auth/session', { signal })); }
  catch (error) { if (error instanceof ApiError && error.code === 'unauthenticated') return null; throw error; }
}
