import { useCallback, useEffect, useState, type FormEvent } from 'react';
import { request } from '../../lib/api';
import { parseNote, type Note } from '../../lib/contracts';
import { useFormState } from '../../lib/useFormState';
export function NotesPage({ online }: { online: boolean }) {
  const [notes, setNotes] = useState<Note[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState('');
  const [editing, setEditing] = useState<Note | null>(null);
  const [title, setTitle] = useState(''); const [body, setBody] = useState('');
  const [notice, setNotice] = useState('');
  const form = useFormState();
  const load = useCallback(async (signal?: AbortSignal) => {
    setLoading(true); setLoadError('');
    try { const value = await request('/notes', { signal }); if (!Array.isArray(value)) throw new Error('Invalid server response'); if (!signal?.aborted) setNotes(value.map(parseNote)); }
    catch (error) { if (!signal?.aborted) setLoadError(error instanceof Error ? error.message : 'Could not load notes'); }
    finally { if (!signal?.aborted) setLoading(false); }
  }, []);
  useEffect(() => { const controller = new AbortController(); void load(controller.signal); return () => controller.abort(); }, [load]);
  function clear() { setEditing(null); setTitle(''); setBody(''); form.clearError(); }
  const save = form.wrapHandler(async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const value = parseNote(await request(editing ? `/notes/${editing.id}` : '/notes', { method: editing ? 'PUT' : 'POST', body: { title, body, ...(editing ? { revision: editing.revision } : {}) } }));
    setNotes(previous => editing ? previous.map(note => note.id === value.id ? value : note) : [value, ...previous]);
    clear(); setNotice('Saved to your account.');
  });
  const remove = form.wrapHandler(async (note: Note) => {
    if (!window.confirm(`Delete “${note.title}”? This cannot be undone.`)) return;
    await request(`/notes/${note.id}`, { method: 'DELETE', body: { revision: note.revision } });
    setNotes(previous => previous.filter(value => value.id !== note.id)); if (editing?.id === note.id) clear(); setNotice('Note deleted.');
  });
  return <section><div className="page-heading"><div><p className="eyebrow">WORKSPACE / EXAMPLE</p><h1>Private notes</h1><p className="subtle">A small example of authenticated, owner-scoped data. Replace it with your product.</p></div><button disabled={!online || loading || form.isLoading} onClick={() => { clear(); void load(); }}>Reload</button></div>
    <div className="workspace-grid"><section className="panel editor"><div className="panel-heading"><h2>{editing ? 'Edit note' : 'New note'}</h2><span className="tag">{notes.length} / 100</span></div><form onSubmit={event => { event.preventDefault(); void save(event).catch(() => {}); }} aria-busy={form.isLoading}>
      <label>Title<input value={title} onChange={event => setTitle(event.target.value)} required maxLength={120} disabled={form.isLoading} /></label>
      <label>Note<textarea value={body} onChange={event => setBody(event.target.value)} rows={9} maxLength={10000} disabled={form.isLoading} placeholder="Start with something worth keeping." /></label>
      {form.error && <p className="error" role="alert">{form.error}</p>}<div className="actions"><button className="primary" disabled={!online || form.isLoading}>{form.isLoading ? 'Saving…' : 'Save note'}</button>{editing && <button type="button" disabled={form.isLoading} onClick={clear}>Cancel edit</button>}</div>
    </form>{notice && <p className="notice" role="status">{notice}</p>}</section>
    <section aria-label="Saved notes" className="notes-list" aria-busy={loading}>{loading ? <div className="panel empty" role="status">Loading your notes…</div> : loadError ? <p className="error panel" role="alert">{loadError}</p> : notes.length === 0 ? <div className="panel empty"><span className="empty-symbol" aria-hidden="true">＋</span><h2>A clear starting point</h2><p>Create a note. Only your account can access it.</p></div> : notes.map(note => <article className="panel note" key={note.id}><div className="panel-heading"><span className="eyebrow">NOTE · REV {note.revision}</span><time dateTime={new Date(note.updated_at * 1000).toISOString()}>{new Date(note.updated_at * 1000).toLocaleDateString()}</time></div><h2>{note.title}</h2><p className="note-body">{note.body || 'No content yet.'}</p><div className="actions"><button disabled={!online || form.isLoading} onClick={() => { setEditing(note); setTitle(note.title); setBody(note.body); form.clearError(); setNotice(''); }}>Edit</button><button className="danger" disabled={!online || form.isLoading} onClick={() => void remove(note).catch(() => {})}>Delete</button></div></article>)}</section></div></section>;
}
