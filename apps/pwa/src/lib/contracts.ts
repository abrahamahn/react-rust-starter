export interface User { id: string; email: string; display_name: string; theme: 'system' | 'light' | 'dark'; email_verified: boolean }
export interface Session { user: User; expires_at: number; idle_timeout_seconds: number }
export interface Note { id: string; title: string; body: string; revision: number; updated_at: number }
export function parseUser(value: unknown): User {
  const v = value as Partial<User> | null;
  if (!v || typeof v.id !== 'string' || typeof v.email !== 'string' || typeof v.display_name !== 'string' || typeof v.email_verified !== 'boolean' || !['system','light','dark'].includes(String(v.theme))) throw new Error('Invalid server response');
  return v as User;
}
export function parseSession(value: unknown): Session {
  const v = value as Partial<Session> | null;
  if (!v || typeof v.expires_at !== 'number' || !Number.isSafeInteger(v.expires_at) || typeof v.idle_timeout_seconds !== 'number') throw new Error('Invalid server response');
  return { user: parseUser(v.user), expires_at: v.expires_at, idle_timeout_seconds: v.idle_timeout_seconds };
}
export function parseNote(value: unknown): Note {
  const v = value as Partial<Note> | null;
  if (!v || typeof v.id !== 'string' || typeof v.title !== 'string' || typeof v.body !== 'string' || typeof v.revision !== 'number' || !Number.isInteger(v.revision) || v.revision < 1 || typeof v.updated_at !== 'number') throw new Error('Invalid server response');
  return v as Note;
}
