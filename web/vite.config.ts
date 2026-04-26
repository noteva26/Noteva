import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

const enableReactCompiler = process.env.REACT_COMPILER === '1'

const reactCompilerConfig = {
  compilationMode: 'annotation',
  panicThreshold: 'none',
}

export default defineConfig({
  plugins: [
    react({
      babel: {
        plugins: enableReactCompiler
          ? [['babel-plugin-react-compiler', reactCompilerConfig]]
          : [],
      },
    }),
    tailwindcss(),
  ],
  base: '/manage/',
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
        manualChunks: {
          // Vendor: React core
          'vendor-react': ['react', 'react-dom', 'react-router-dom'],
          // UI libs
          'vendor-ui': ['motion', 'sonner', 'lucide-react'],
          // Charts (heavy)
          'vendor-charts': ['recharts'],
          // Code editor
          'vendor-codemirror': [
            '@codemirror/commands',
            '@codemirror/lang-markdown',
            '@codemirror/language',
            '@codemirror/state',
            '@codemirror/theme-one-dark',
            '@codemirror/view',
          ],
          // Markdown rendering
          'vendor-markdown': ['react-markdown', 'react-syntax-highlighter'],
        },
      },
    },
  },
  server: {
    proxy: {
      '/api': 'http://localhost:8080',
      '/uploads': 'http://localhost:8080',
    },
  },
})
