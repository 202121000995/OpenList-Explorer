import { fileURLToPath, URL } from 'node:url'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: false
  },
  build: {
    chunkSizeWarningLimit: 1300,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes('node_modules')) return
          if (id.includes('naive-ui') || id.includes('@css-render') || id.includes('vueuc')) return 'ui'
          if (id.includes('@tauri-apps')) return 'tauri'
          if (id.includes('qrcode')) return 'qrcode'
          if (id.includes('axios')) return 'network'
          if (id.includes('vue') || id.includes('pinia') || id.includes('@vueuse')) return 'vue'
          return 'vendor'
        }
      }
    }
  },
  clearScreen: false
})
