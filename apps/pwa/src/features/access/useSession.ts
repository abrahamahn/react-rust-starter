import { useCallback, useEffect, useRef, useState } from 'react';
import { currentSession, request } from '../../lib/api';
import type { Session } from '../../lib/contracts';
export function useSession() {
  const [session, setSession] = useState<Session | null>(null);
  const [checking, setChecking] = useState(true);
  const [error, setError] = useState('');
  const pending = useRef<AbortController | null>(null);
  const generation = useRef(0);
  const accept = useCallback((value: Session | null) => {
    generation.current++; pending.current?.abort(); setSession(value); setChecking(false); setError('');
  }, []);
  const refresh = useCallback(async () => {
    const version = ++generation.current;
    pending.current?.abort(); const controller = new AbortController(); pending.current = controller;
    try { const value = await currentSession(controller.signal); if (!controller.signal.aborted && version === generation.current) { setSession(value); setError(''); } }
    catch (reason) { if (!controller.signal.aborted && version === generation.current) setError(reason instanceof Error ? reason.message : 'Session check failed'); }
    finally { if (version === generation.current) setChecking(false); }
  }, []);
  useEffect(() => {
    void refresh();
    const focus = () => { if (document.visibilityState === 'visible') void refresh(); };
    // Revocation invalidates in-flight reads too; otherwise a late success can
    // resurrect stale UI state after another request has reported a 401.
    const unauthorized = () => accept(null);
    window.addEventListener('focus', focus); window.addEventListener('online', focus); document.addEventListener('visibilitychange', focus);
    window.addEventListener('starter:unauthenticated', unauthorized);
    return () => { generation.current++; pending.current?.abort(); window.removeEventListener('focus', focus); window.removeEventListener('online', focus); document.removeEventListener('visibilitychange', focus); window.removeEventListener('starter:unauthenticated', unauthorized); };
  }, [refresh, accept]);
  const logout = useCallback(async (all = false) => { await request(`/auth/${all ? 'logout-all' : 'logout'}`, { method: 'POST' }); accept(null); }, [accept]);
  return { session, checking, error, accept, refresh, logout };
}
