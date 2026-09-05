import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createFormHandler } from '../apps/pwa/src/lib/createFormHandler.ts';
import { parseUser, parseSession, parseNote } from '../apps/pwa/src/lib/contracts.ts';
import { request, currentSession, ApiError } from '../apps/pwa/src/lib/api.ts';

test('BSLT form handler tracks success and always releases pending state', async () => {
  const pending: boolean[] = []; const errors: (string | null)[] = []; const events: string[] = [];
  const run = createFormHandler(v => pending.push(v), v => errors.push(v))(async (n: number) => n + 1, { onStart: () => events.push('start'), onSuccess: () => events.push('success'), onFinally: () => events.push('finally') });
  assert.equal(await run(2), 3); assert.deepEqual(pending, [true, false]); assert.deepEqual(errors, [null]); assert.deepEqual(events, ['start', 'success', 'finally']);
});
test('form failures are visible and remain failures to the caller', async () => {
  const pending: boolean[] = []; const errors: (string | null)[] = [];
  const run = createFormHandler(v => pending.push(v), v => errors.push(v))(async () => { throw new Error('Rejected by server'); });
  await assert.rejects(run(undefined), /Rejected by server/);
  assert.deepEqual(pending, [true,false]); assert.deepEqual(errors, [null,'Rejected by server']);
});
test('a throwing start callback cannot leave the form busy', async () => {
  const pending: boolean[] = [];
  const run = createFormHandler(v => pending.push(v), () => {})(async () => 1, { onStart: () => { throw new Error('callback'); } });
  await assert.rejects(run(undefined), /callback/); assert.deepEqual(pending, [true,false]);
});
test('wire contracts reject malformed server data', () => {
  const user = { id: 'u', email: 'a@example.test', display_name: '', theme: 'system', email_verified: true };
  assert.equal(parseUser(user).id, 'u');
  assert.equal(parseSession({user, expires_at: 1000, idle_timeout_seconds: 43200}).expires_at, 1000);
  assert.equal(parseNote({id:'n',title:'N',body:'',revision:1,updated_at:1000}).revision,1);
  for (const value of [null,[],{}, { ...user, theme:'unknown' }, { ...user, theme:{toString:()=> 'system'} }]) assert.throws(() => parseUser(value));
  assert.throws(() => parseSession({user,expires_at:'1000'}));
  for (const value of [NaN,Infinity,-1,0,1.5]) assert.throws(() => parseSession({user,expires_at:1000,idle_timeout_seconds:value}));
  for (const value of [NaN,Infinity,-1,1.5]) assert.throws(() => parseNote({id:'n',title:'N',body:'',revision:1,updated_at:value}));
  assert.throws(() => parseNote({id:'n',title:'N',body:'',revision:0,updated_at:1000}));
});
test('write network failure is never automatically retried', async () => {
  const original=globalThis.fetch; let calls=0;
  globalThis.fetch=async (_input,init)=> { calls++; assert.equal(init?.credentials,'same-origin'); assert.equal(init?.cache,'no-store'); throw new Error('connection failed'); };
  try { await assert.rejects(request('/notes',{method:'POST',body:{title:'Example',body:''}}),/submitted change/); assert.equal(calls,1); }
  finally { globalThis.fetch=original; }
});
test('session reads cannot broadcast stale revocation; protected requests still can', async () => {
  const originalFetch=globalThis.fetch; const originalWindow=Object.getOwnPropertyDescriptor(globalThis,'window');
  const events=new EventTarget(); let notifications=0;
  events.addEventListener('starter:unauthenticated',()=>notifications++);
  Object.defineProperty(globalThis,'window',{value:events,configurable:true});
  globalThis.fetch=async()=>new Response(JSON.stringify({error:{code:'unauthenticated'}}),{status:401});
  try {
    assert.equal(await currentSession(),null); assert.equal(notifications,0);
    await assert.rejects(request('/notes'),(error:unknown)=>error instanceof ApiError && error.status===401); assert.equal(notifications,1);
    const controller=new AbortController(); controller.abort();
    await assert.rejects(request('/notes',{signal:controller.signal})); assert.equal(notifications,1);
  } finally { globalThis.fetch=originalFetch; if(originalWindow) Object.defineProperty(globalThis,'window',originalWindow); else Reflect.deleteProperty(globalThis,'window'); }
});
