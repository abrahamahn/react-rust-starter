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
    try { const value = await currentSession(controller.signal); if (version === generation.current) { setSession(value); setError(''); } }
    catch (reason) { if (!controller.signal.aborted && version === generation.current) setError(reason instanceof Error ? reason.message : 'Session check failed'); }
    finally { if (version === generation.current) setChecking(false); }
  }, []);
  useEffect(() => {
    void refresh();
    const focus = () => { if (document.visibilityState === 'visible') void refresh(); };
    const unauthorized = () => { setSession(null); };
    window.addEventListener('focus', focus); document.addEventListener('visibilitychange', focus);
    window.addEventListener('starter:unauthenticated', unauthorized);
    return () => { generation.current++; pending.current?.abort(); window.removeEventListener('focus', focus); document.removeEventListener('visibilitychange', focus); window.removeEventListener('starter:unauthenticated', unauthorized); };
  }, [refresh]);
  const logout = useCallback(async (all = false) => { await request(`/auth/${all ? 'logout-all' : 'logout'}`, { method: 'POST' }); accept(null); }, [accept]);
  return { session, checking, error, accept, refresh, logout };
}
