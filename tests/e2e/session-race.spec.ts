import { test, expect } from '@playwright/test';

// Deliberately synthetic scheduling regression, separate from the real SMTP/DB
// lifecycle tests. Even a transport that ignores abort must not revive a session.
test('late session read cannot restore UI state after revocation', async ({ page }) => {
  const session = { user: { id:'race-test',email:'race@example.test',display_name:'Race test',theme:'system',email_verified:false }, expires_at:2000000000,idle_timeout_seconds:43200 };
  await page.route('**/api/auth/session',route=>route.fulfill({status:200,contentType:'application/json',body:JSON.stringify(session)}));
  await page.goto('/');
  await expect(page.getByRole('navigation',{name:'Workspace'})).toBeVisible();
  await page.evaluate(value => {
    const browser=window as typeof window & { finishLateRead?:()=>void; lateReadStarted?:boolean };
    const original=window.fetch;
    window.fetch=(input,options)=>String(input).endsWith('/api/auth/session') ? new Promise<Response>(resolve=>{
      browser.lateReadStarted=true;
      browser.finishLateRead=()=>resolve(new Response(JSON.stringify(value),{status:200,headers:{'Content-Type':'application/json'}}));
    }) : original(input,options);
    window.dispatchEvent(new Event('focus'));
  },session);
  await expect.poll(()=>page.evaluate(()=>(window as typeof window & {lateReadStarted?:boolean}).lateReadStarted)).toBe(true);
  await page.evaluate(()=>window.dispatchEvent(new Event('starter:unauthenticated')));
  await expect(page.getByRole('heading',{name:'Welcome back',exact:true})).toBeVisible();
  await page.evaluate(async()=>{
    (window as typeof window & {finishLateRead?:()=>void}).finishLateRead?.();
    await new Promise(resolve=>setTimeout(resolve,50));
  });
  await expect(page.getByRole('heading',{name:'Welcome back',exact:true})).toBeVisible();
  await expect(page.getByRole('navigation',{name:'Workspace'})).toHaveCount(0);
});
