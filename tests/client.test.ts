import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createFormHandler } from '../apps/pwa/src/lib/createFormHandler.ts';
import { parseUser, parseSession, parseNote } from '../apps/pwa/src/lib/contracts.ts';

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
  for (const value of [null,{}, { ...user, theme:'unknown' }]) assert.throws(() => parseUser(value));
  assert.throws(() => parseSession({user,expires_at:'1000'}));
  assert.throws(() => parseNote({id:'n',title:'N',body:'',revision:0,updated_at:1000}));
});
