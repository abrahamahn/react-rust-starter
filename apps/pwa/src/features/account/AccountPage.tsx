import { useState, type FormEvent } from 'react';
import { request } from '../../lib/api';
import { parseUser, type User } from '../../lib/contracts';
import { useFormState } from '../../lib/useFormState';
export function AccountPage({ user, onUser, onSignedOut, onRevoke, online }: { user: User; onUser: (value: User) => void; onSignedOut: () => void; onRevoke: () => Promise<void>; online: boolean }) {
  const [notice, setNotice] = useState('');
  const profile = useFormState(); const security = useFormState();
  const save = profile.wrapHandler(async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault(); const data = new FormData(event.currentTarget);
    onUser(parseUser(await request('/account', { method: 'PATCH', body: { display_name: String(data.get('display_name')), theme: String(data.get('theme')) } })));
    setNotice('Account settings saved.');
  });
  const change = security.wrapHandler(async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault(); const data = new FormData(event.currentTarget);
    const password = String(data.get('password'));
    if ([...password].length < 15 || [...password].length > 128) throw new Error('Use 15–128 characters.');
    if (password !== data.get('confirm')) throw new Error('The passwords do not match.');
    await request('/auth/change-password', { method: 'POST', body: { current_password: String(data.get('current_password')), password } });
    onSignedOut();
  });
  const revoke = security.wrapHandler(async () => { if (window.confirm('Sign out every existing session, including this one?')) await onRevoke(); });
  return <section><div className="page-heading"><div><p className="eyebrow">IDENTITY / PREFERENCES</p><h1>Your account</h1><p className="subtle">A few essentials. Kept in your account, not in browser storage.</p></div></div><div className="workspace-grid">
    <section className="panel"><h2>Profile & appearance</h2><dl className="details"><div><dt>Email</dt><dd>{user.email}</dd></div><div><dt>Status</dt><dd>{user.email_verified ? 'Verified' : 'Awaiting verification'}</dd></div></dl>
    <form onSubmit={event => { event.preventDefault(); void save(event).catch(() => {}); }}><label>Display name<input name="display_name" defaultValue={user.display_name} maxLength={80} autoComplete="name" disabled={profile.isLoading} /></label><label>Appearance<select name="theme" defaultValue={user.theme} disabled={profile.isLoading}><option value="system">System</option><option value="dark">Dark</option><option value="light">Light</option></select></label>{profile.error && <p className="error" role="alert">{profile.error}</p>}<button className="primary" disabled={!online || profile.isLoading}>Save settings</button></form>{notice && <p className="notice" role="status">{notice}</p>}</section>
    <section className="panel"><h2>Session security</h2><p className="subtle">Changing your password signs out every device. Sign in again with the new password.</p><form onSubmit={event => { event.preventDefault(); void change(event).catch(() => {}); }}><label>Current password<input name="current_password" type="password" autoComplete="current-password" required maxLength={256} disabled={security.isLoading} /></label><label>New password<input name="password" type="password" autoComplete="new-password" required maxLength={256} disabled={security.isLoading} /></label><label>Confirm new password<input name="confirm" type="password" autoComplete="new-password" required maxLength={256} disabled={security.isLoading} /></label>{security.error && <p className="error" role="alert">{security.error}</p>}<button className="primary" disabled={!online || security.isLoading}>Change password</button></form><hr /><button className="danger" disabled={!online || security.isLoading} onClick={() => void revoke(undefined).catch(() => {})}>Sign out all devices</button></section></div></section>;
}
