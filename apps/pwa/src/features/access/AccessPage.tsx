import { useState, type FormEvent } from 'react';
import { request } from '../../lib/api';
import { parseSession, type Session } from '../../lib/contracts';
import { useFormState } from '../../lib/useFormState';
export interface LinkAction { kind: 'verify' | 'reset'; token: string }
export function AccessPage({ onSession, action, onActionDone, online }: { onSession: (value: Session | null) => void; action: LinkAction | null; onActionDone: () => void; online: boolean }) {
  const [mode, setMode] = useState<'login' | 'register' | 'forgot'>('login');
  const [notice, setNotice] = useState('');
  const form = useFormState();
  const title = action?.kind === 'verify' ? 'Verify your email' : action?.kind === 'reset' ? 'Choose a new password' : mode === 'register' ? 'Create your account' : mode === 'forgot' ? 'Reset your password' : 'Welcome back';
  const submit = form.wrapHandler(async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault(); setNotice('');
    const values = new FormData(event.currentTarget);
    const password = String(values.get('password') ?? '');
    if ((mode === 'register' && !action) || action?.kind === 'reset') {
      if ([...password].length < 15 || [...password].length > 128) throw new Error('Use 15–128 characters for your password.');
      if (password !== values.get('confirm')) throw new Error('The passwords do not match.');
    }
    if (action?.kind === 'verify') {
      await request('/auth/verify', { method: 'POST', body: { token: action.token } });
      onActionDone(); setNotice('Email verified. You can now use your workspace.'); return;
    }
    if (action?.kind === 'reset') {
      await request('/auth/reset-password', { method: 'POST', body: { token: action.token, password } });
      onSession(null); onActionDone(); setMode('login'); setNotice('Password updated. Sign in with your new password.'); return;
    }
    const email = String(values.get('email') ?? '');
    if (mode === 'forgot') {
      await request('/auth/forgot-password', { method: 'POST', body: { email } });
      setNotice('If this account is eligible, a reset link will arrive by email.'); return;
    }
    onSession(parseSession(await request(`/auth/${mode}`, { method: 'POST', body: { email, password, remember_me: values.get('remember') === 'on' } })));
  });
  function change(next: typeof mode) { setMode(next); form.clearError(); setNotice(''); }
  return <section className="auth-layout">
    <div className="intro"><p className="eyebrow">A SMALL, REAL FOUNDATION</p><h1>Your next idea.<br /><span>Already connected.</span></h1><p>React on the surface. Rust underneath. One clear starting point for the application you want to build.</p><div className="stack-strip"><span>REACT</span><i /><span>RUST</span><i /><span>POSTGRES</span></div><p className="subtle">Accounts, sessions, settings and a private notes example. Nothing product-specific.</p></div>
    <div className="panel auth-panel"><p className="eyebrow">ACCOUNT / ACCESS</p><h2>{title}</h2><p className="subtle">{action ? 'Use this link once. Your credentials are not stored in the browser.' : 'Your account is managed by the Rust server.'}</p>
      <form key={action?.kind ?? mode} onSubmit={event => { event.preventDefault(); void submit(event).catch(() => {}); }} aria-busy={form.isLoading}>
        {!action && <label>Email<input name="email" type="email" autoComplete="username" required maxLength={254} disabled={form.isLoading} /></label>}
        {action?.kind !== 'verify' && (action?.kind === 'reset' || mode !== 'forgot') && <label>Password<input name="password" type="password" autoComplete={action || mode === 'register' ? 'new-password' : 'current-password'} required maxLength={256} disabled={form.isLoading} /></label>}
        {(action?.kind === 'reset' || (!action && mode === 'register')) && <><label>Confirm password<input name="confirm" type="password" autoComplete="new-password" required maxLength={256} disabled={form.isLoading} /></label><p className="hint">15–128 characters. Spaces are preserved.</p></>}
        {!action && mode !== 'forgot' && <label className="checkbox"><input name="remember" type="checkbox" disabled={form.isLoading} /> Keep me signed in for up to 30 days</label>}
        {form.error && <p className="error" role="alert">{form.error}</p>}
        <button className="primary" disabled={form.isLoading || !online}>{form.isLoading ? 'Working…' : action?.kind === 'verify' ? 'Confirm email' : action?.kind === 'reset' ? 'Set new password' : mode === 'register' ? 'Create account' : mode === 'forgot' ? 'Send reset link' : 'Sign in'}</button>
      </form>
      {notice && <p role="status" className="notice">{notice}</p>}
      {!action ? <div className="auth-links"><button className="text-button" disabled={form.isLoading} onClick={() => change(mode === 'login' ? 'register' : 'login')}>{mode === 'login' ? 'Create an account' : 'Back to sign in'}</button>{mode === 'login' && <button className="text-button" disabled={form.isLoading} onClick={() => change('forgot')}>Forgot password?</button>}</div> : <button className="text-button" disabled={form.isLoading} onClick={onActionDone}>Back to app</button>}
    </div>
  </section>;
}
