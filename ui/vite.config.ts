import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')

  const askServiceUrl = env.VITE_ASK_SERVICE_URL || 'http://localhost:17001'
  const notifyServiceUrl = env.VITE_NOTIFY_SERVICE_URL || 'http://localhost:17004'
  const orchestratorServiceUrl = env.VITE_ORCHESTRATOR_SERVICE_URL || 'http://localhost:17006'

  const src = (path: string) => fileURLToPath(new URL(`./src/${path}`, import.meta.url))

  return {
    plugins: [react(), tailwindcss()],
    resolve: {
      alias: {
        '@': src(''),
        '@/components': src('components'),
        '@/hooks': src('hooks'),
        '@/layouts': src('layouts'),
        '@/pages': src('pages'),
        '@/services': src('services'),
        '@/types': src('types'),
        '@/utils': src('utils'),
        '@/stores': src('stores'),
      },
    },
    server: {
      port: 5173,
      proxy: {
        '/api/ask': {
          target: askServiceUrl,
          changeOrigin: true,
          rewrite: (path) => path.replace(/^\/api\/ask/, ''),
        },
        '/api/notify': {
          target: notifyServiceUrl,
          changeOrigin: true,
          rewrite: (path) => path.replace(/^\/api\/notify/, ''),
        },
        '/api/orchestrator': {
          target: orchestratorServiceUrl,
          changeOrigin: true,
          rewrite: (path) => path.replace(/^\/api\/orchestrator/, ''),
        },
      },
    },
  }
})
