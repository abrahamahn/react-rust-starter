import { useEffect, useRef, useState } from 'react';
export function PwaStatus() {
  const [waiting, setWaiting] = useState<ServiceWorker | null>(null);
  const reloadRequested = useRef(false);
  useEffect(() => {
    if (!import.meta.env.PROD || !('serviceWorker' in navigator)) return;
    let active = true;
    const changed = () => { if (reloadRequested.current) location.reload(); };
    navigator.serviceWorker.addEventListener('controllerchange', changed);
    void navigator.serviceWorker.register('/sw.js').then(registration => {
      if (!active) return;
      if (registration.waiting) setWaiting(registration.waiting);
      registration.addEventListener('updatefound', () => {
        const worker = registration.installing;
        worker?.addEventListener('statechange', () => { if (active && worker.state === 'installed' && navigator.serviceWorker.controller) setWaiting(worker); });
      });
    }).catch(() => { /* The online app remains usable when installation is unavailable. */ });
    return () => { active = false; navigator.serviceWorker.removeEventListener('controllerchange', changed); };
  }, []);
  return waiting ? <div className="banner" role="status">An app update is ready. Save your work before reloading.<button onClick={() => { reloadRequested.current = true; waiting.postMessage({ type: 'SKIP_WAITING' }); }}>Update app</button></div> : null;
}
