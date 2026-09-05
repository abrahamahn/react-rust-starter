import { useEffect, useState } from 'react';
import { appConfig } from './config';
import { AccessPage, type LinkAction } from '../features/access/AccessPage';
import { useSession } from '../features/access/useSession';
import { AccountPage } from '../features/account/AccountPage';
import { NotesPage } from '../features/notes/NotesPage';
import { request } from '../lib/api';
import { useFormState } from '../lib/useFormState';
import { PwaStatus } from './PwaStatus';
function readAction(): LinkAction | null {
  const match = /^#(verify|reset)=([a-fA-F0-9]{64})$/.exec(window.location.hash);
  return match ? { kind: match[1] as LinkAction['kind'], token: match[2] } : null;
}
const initialAction = readAction();
export function App() {
  const auth = useSession();
  const [action, setAction] = useState<LinkAction | null>(initialAction);
  const [tab, setTab] = useState(location.hash === '#/account' ? 'account' : 'notes');
  const [online, setOnline] = useState(navigator.onLine);
  const [notice, setNotice] = useState('');
  const form = useFormState();
  useEffect(() => {
    if (initialAction) history.replaceState(null, '', location.pathname);
    const route = () => { const next = readAction(); if (next) { setAction(next); history.replaceState(null, '', location.pathname); } else setTab(location.hash === '#/account' ? 'account' : 'notes'); };
    const network = () => setOnline(navigator.onLine);
    window.addEventListener('hashchange', route); window.addEventListener('online', network); window.addEventListener('offline', network);
    document.title = appConfig.name;
    return () => { window.removeEventListener('hashchange', route); window.removeEventListener('online', network); window.removeEventListener('offline', network); };
  }, []);
  useEffect(() => { document.documentElement.dataset.theme = auth.session?.user.theme ?? 'system'; }, [auth.session?.user.theme]);
  const signout = form.wrapHandler(async () => { await auth.logout(); setNotice(''); });
  const resend = form.wrapHandler(async () => {
    if (!auth.session) return;
    await request('/auth/request-verification', { method: 'POST', body: { email: auth.session.user.email } });
    setNotice('If eligible, a new verification link will arrive by email.');
  });
  return <div className="app"><a className="skip-link" href="#main">Skip to content</a><header className="topbar"><a className="brand" href="#/notes"><span className="brand-icon" aria-hidden="true">r</span><span>{appConfig.name}</span></a><div className="header-right"><span className="connection"><i className={online ? 'online' : 'offline'} />{online ? 'CONNECTED' : 'OFFLINE'}</span>{auth.session && <button disabled={!online || form.isLoading} onClick={() => void signout(undefined).catch(() => {})}>Sign out</button>}</div></header>
    <PwaStatus />{!online && <div className="banner" role="status">You are offline. Changes and account actions require a connection.</div>}
    <main id="main" tabIndex={-1}>{auth.error && <div className="error banner" role="alert">{auth.error}<button onClick={() => void auth.refresh()}>Retry session check</button></div>}{form.error && <p className="error banner" role="alert">{form.error}</p>}
      {action ? <AccessPage action={action} online={online} onSession={auth.accept} onActionDone={() => { setAction(null); void auth.refresh(); }} /> : auth.checking ? <div className="panel empty" role="status">Checking your session…</div> : !auth.session ? <AccessPage action={null} online={online} onSession={auth.accept} onActionDone={() => {}} /> : <div className="workspace"><nav aria-label="Workspace"><a href="#/notes" aria-current={tab === 'notes' ? 'page' : undefined}>Notes</a><a href="#/account" aria-current={tab === 'account' ? 'page' : undefined}>Account</a><span className="nav-account">{auth.session.user.display_name || auth.session.user.email}</span></nav>
        {!auth.session.user.email_verified && <section className="panel verification"><div><p className="eyebrow">ONE MORE STEP</p><h2>Check your inbox</h2><p>Verify {auth.session.user.email} before creating notes. Development emails appear in your local Mailpit inbox.</p></div><div className="actions"><button disabled={!online || form.isLoading} onClick={() => void resend(undefined).catch(() => {})}>Resend verification</button><button className="primary" disabled={!online} onClick={() => void auth.refresh()}>I have verified my email</button></div>{notice && <p role="status" className="notice">{notice}</p>}</section>}
        {tab === 'account' ? <AccountPage key={auth.session.user.id} user={auth.session.user} online={online} onUser={user => { if (auth.session) auth.accept({ ...auth.session, user }); }} onSignedOut={() => auth.accept(null)} onRevoke={() => auth.logout(true)} /> : auth.session.user.email_verified ? <NotesPage key={auth.session.user.id} online={online} /> : null}
      </div>}
    </main><footer className="footer"><span>REACT + RUST</span><span>A minimal foundation · Built for your next product</span></footer></div>;
}
