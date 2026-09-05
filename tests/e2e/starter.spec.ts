import { test, expect, type APIRequestContext, type Page } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { readFile } from 'node:fs/promises';
const password = 'browser-only test password';
const replacement = 'replacement browser password';
async function link(api: APIRequestContext, email: string, kind: 'verify' | 'reset') {
  let found = '';
  await expect.poll(async () => {
    const data = await (await api.get('http://127.0.0.1:8025/api/v1/messages?limit=100')).json();
    for (const message of data.messages ?? []) {
      if (!message.To?.some((to: { Address: string }) => to.Address === email)) continue;
      const full = await (await api.get(`http://127.0.0.1:8025/api/v1/message/${message.ID}`)).json();
      const match = new RegExp(`http://127\\.0\\.0\\.1:5178/#${kind}=[a-f0-9]{64}`).exec(full.Text ?? '');
      if (match) { found = match[0]; break; }
    }
    return Boolean(found);
  }).toBeTruthy();
  return found;
}
async function signIn(page: Page, email: string, value = password) {
  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill(value);
  await page.getByRole('button', { name: 'Sign in', exact: true }).click();
}
test('real account, SMTP verification, notes, settings, reset and global logout', async ({ page, request, browser }) => {
  const email = `browser-${randomUUID()}@example.test`;
  const pageErrors: string[] = []; page.on('pageerror', error => pageErrors.push(error.message));
  await page.goto('/');
  await page.getByRole('button', { name: 'Create an account', exact: true }).click();
  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByLabel('Password', { exact: true }).fill(password);
  await page.getByLabel('Confirm password', { exact: true }).fill(password);
  await page.getByRole('button', { name: 'Create account', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Check your inbox' })).toBeVisible();
  await page.goto(await link(request,email,'verify'));
  await page.getByRole('button', { name: 'Confirm email', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Private notes' })).toBeVisible();
  await expect(page).toHaveURL('http://127.0.0.1:5178/');
  await page.getByLabel('Title', { exact: true }).fill('First private note');
  await page.getByLabel('Note', { exact: true }).fill('Stored by Rust, not simulated in the browser.');
  await page.getByRole('button', { name: 'Save note' }).click();
  await expect(page.getByRole('heading', { name: 'First private note' })).toBeVisible();
  await page.reload();
  await expect(page.getByRole('heading', { name: 'First private note' })).toBeVisible();
  await page.getByRole('button', { name: 'Edit', exact: true }).click();
  await page.getByLabel('Title', { exact: true }).fill('Updated private note');
  await page.getByRole('button', { name: 'Save note' }).click();
  await expect(page.getByRole('heading', { name: 'Updated private note' })).toBeVisible();
  page.once('dialog', dialog => dialog.accept());
  await page.getByRole('button', { name: 'Delete', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'A clear starting point' })).toBeVisible();
  await page.getByRole('link', { name: 'Account', exact: true }).click();
  await page.getByLabel('Display name').fill('Starter tester');
  await page.getByLabel('Appearance').selectOption('light');
  await page.getByRole('button', { name: 'Save settings' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme','light');
  await page.getByRole('button', { name: 'Sign out', exact: true }).click();
  await signIn(page,email,'incorrect password');
  await expect(page.getByRole('alert')).toContainText('incorrect');
  await page.getByRole('button', { name: 'Forgot password?' }).click();
  await page.getByLabel('Email', { exact: true }).fill(email);
  await page.getByRole('button', { name: 'Send reset link' }).click();
  await page.goto(await link(request,email,'reset'));
  await page.getByLabel('Password', { exact: true }).fill(replacement);
  await page.getByLabel('Confirm password', { exact: true }).fill(replacement);
  await page.getByRole('button', { name: 'Set new password' }).click();
  await expect(page.getByRole('heading', { name: 'Welcome back' })).toBeVisible();
  await signIn(page,email,replacement);
  await expect(page.getByRole('navigation', { name: 'Workspace' })).toBeVisible();
  const other = await browser.newContext({ baseURL:'http://127.0.0.1:5178' });
  const otherPage = await other.newPage(); await otherPage.goto('/'); await signIn(otherPage,email,replacement);
  await expect(otherPage.getByRole('heading', { name: 'Private notes' })).toBeVisible();
  await page.getByRole('link', { name:'Account',exact:true }).click();
  page.once('dialog',dialog=>dialog.accept());
  await page.getByRole('button',{name:'Sign out all devices'}).click();
  await expect(page.getByRole('heading',{name:'Welcome back'})).toBeVisible();
  await otherPage.reload(); await expect(otherPage.getByRole('heading',{name:'Welcome back'})).toBeVisible();
  await other.close();
  expect(await page.evaluate(() => Object.keys(localStorage))).toEqual([]);
  expect(pageErrors).toEqual([]);
});
test('responsive login, install resources, and offline cache excludes all private API data', async ({ page, context, request }) => {
  await page.setViewportSize({width:375,height:812}); await page.goto('/');
  await expect(page.getByRole('heading',{name:'Welcome back'})).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBeTruthy();
  const manifest = await (await request.get('/manifest.webmanifest')).json();
  expect(manifest.display).toBe('standalone');
  for (const size of [192,512]) {
    const response = await request.get(`/icon-${size}.png`); expect(response.ok()).toBeTruthy();
    const png = await response.body(); expect(png.readUInt32BE(16)).toBe(size); expect(png.readUInt32BE(20)).toBe(size);
  }
  await page.evaluate(async()=>{ await navigator.serviceWorker.ready; });
  await expect.poll(()=>page.evaluate(()=>Boolean(navigator.serviceWorker.controller))).toBeTruthy();
  await page.evaluate(async()=>{ await fetch('/api/auth/session',{cache:'no-store'}); });
  const cached = await page.evaluate(async()=>{const names=await caches.keys();return (await Promise.all(names.map(async name=>(await (await caches.open(name)).keys()).map(r=>new URL(r.url).pathname)))).flat();});
  expect(cached.some(path=>path.startsWith('/api'))).toBeFalsy();
  await page.screenshot({path:'test-results/starter-mobile.png',fullPage:true});
  await context.setOffline(true); await page.reload();
  await expect(page.getByRole('heading',{name:'You are offline.'})).toBeVisible();
  await context.setOffline(false); await page.getByRole('link',{name:'Try again'}).click();
  await expect(page.getByRole('heading',{name:'Welcome back'})).toBeVisible();
});
