import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { root } from './env.mjs';
const [name, slug, flag] = process.argv.slice(2);
if (!name || name.length > 60 || /[<>\u0000-\u001f]/.test(name) || !slug || !/^[a-z][a-z0-9-]{2,39}$/.test(slug) || (flag && flag !== '--apply')) {
  throw new Error('Usage: pnpm starter:rename "Display Name" app-slug [--apply]');
}
const manifestPath = join(root,'apps/pwa/public/manifest.webmanifest');
const manifest = JSON.parse(readFileSync(manifestPath,'utf8'));
manifest.name = name; manifest.short_name = name.slice(0,12); manifest.id = `/${slug}`;
const html = readFileSync(join(root,'apps/pwa/index.html'),'utf8').replace(/<title>[^<]*<\/title>/, `<title>${name.replaceAll('&','&amp;')}</title>`);
console.log(`Brand name: ${name}; app ID: /${slug}. ${flag ? 'Applying.' : 'Dry run; add --apply to write.'}`);
if (flag) {
  writeFileSync(manifestPath, JSON.stringify(manifest,null,2)+'\n');
  writeFileSync(join(root,'apps/pwa/src/app/config.ts'), `export const appConfig = ${JSON.stringify({name,slug})} as const;\n`);
  writeFileSync(join(root,'apps/pwa/index.html'),html);
}
console.log('Only display identity changes. Configure a NEW database, environment, domain, mail sender and deployment target separately.');
