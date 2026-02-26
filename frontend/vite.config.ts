import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { resolve } from 'path';

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      $lib: resolve(__dirname, 'src/lib'),
    },
  },
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'main.html'),
        overlay: resolve(__dirname, 'overlay.html'),
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
});
