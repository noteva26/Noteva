import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes('node_modules')) return undefined

          if (/[\\/]node_modules[\\/](react|react-dom|scheduler|react-router|react-router-dom)[\\/]/.test(id)) {
            return 'react-vendor'
          }

          if (id.includes('node_modules/motion')) {
            return 'motion-vendor'
          }

          if (
            /[\\/]node_modules[\\/](@radix-ui|lucide-react|sonner|next-themes)[\\/]/.test(id)
          ) {
            return 'ui-vendor'
          }

          return 'vendor'
        },
      },
    },
  },
  server: {
    proxy: {
      '/api': 'http://localhost:8080',
      '/uploads': 'http://localhost:8080',
      '/noteva-sdk.js': 'http://localhost:8080',
      '/noteva-sdk.css': 'http://localhost:8080',
    },
  },
})
