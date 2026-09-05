export interface User { id: string; email: string; display_name: string; theme: 'system' | 'light' | 'dark'; email_verified: boolean }
export interface Session { user: User; expires_at: number; idle_timeout_seconds: number }
export interface Note { id: string; title: string; body: string; revision: number; updated_at: number }
function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid server response');
  return value as Record<string, unknown>;
}
function integer(value: unknown, minimum = 0): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < minimum) throw new Error('Invalid server response');
  return value;
}
export function parseUser(value: unknown): User {
  const v = object(value);
  if (typeof v.id !== 'string' || !v.id || typeof v.email !== 'string' || typeof v.display_name !== 'string' || typeof v.email_verified !== 'boolean' || (v.theme !== 'system' && v.theme !== 'light' && v.theme !== 'dark')) throw new Error('Invalid server response');
  return { id: v.id, email: v.email, display_name: v.display_name, theme: v.theme, email_verified: v.email_verified };
}
export function parseSession(value: unknown): Session {
  const v = object(value);
  return { user: parseUser(v.user), expires_at: integer(v.expires_at, 1), idle_timeout_seconds: integer(v.idle_timeout_seconds, 1) };
}
export function parseNote(value: unknown): Note {
  const v = object(value);
  if (typeof v.id !== 'string' || !v.id || typeof v.title !== 'string' || typeof v.body !== 'string') throw new Error('Invalid server response');
  return { id: v.id, title: v.title, body: v.body, revision: integer(v.revision, 1), updated_at: integer(v.updated_at) };
}
