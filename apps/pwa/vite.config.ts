import { defineConfig, loadEnv } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';
export default defineConfig(({ mode }) => {
  const env = { ...loadEnv(mode, fileURLToPath(new URL('../..', import.meta.url)), ''), ...process.env };
  const target = `http://${env.APP_BIND || '127.0.0.1:8088'}`;
  const proxy = { '/api': { target, changeOrigin: false }, '/health': { target, changeOrigin: false } };
  return { plugins: [react()], server: { host: '127.0.0.1', port: 5178, strictPort: true, proxy },
    preview: { host: '127.0.0.1', port: 5178, strictPort: true, proxy }, build: { sourcemap: false } };
});
