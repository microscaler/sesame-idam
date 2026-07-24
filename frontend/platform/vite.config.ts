import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';

export default defineConfig({
  plugins: [solid()],
  server: {
    // Dev: proxy the IDAM API so the browser sees one origin (no CORS in dev;
    // prod uses the A5 allow-list).
    proxy: {
      '/idam': {
        target: process.env.VITE_IDAM_PROXY ?? 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
  build: { target: 'es2022' },
});
