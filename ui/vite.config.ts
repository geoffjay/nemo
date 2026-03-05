import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { resolve } from 'path'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')

  const askServiceUrl = env.VITE_ASK_SERVICE_URL || 'http://localhost:17001'
  const notifyServiceUrl = env.VITE_NOTIFY_SERVICE_URL || 'http://localhost:17004'
  const orchestratorServiceUrl = env.VITE_ORCHESTRATOR_SERVICE_URL || 'http://localhost:17006'

  return {
    plugins: [react(), tailwindcss()],
    resolve: {
      alias: {
        '@': resolve(__dirname, './src'),
        '@/components': resolve(__dirname, './src/components'),
        '@/hooks': resolve(__dirname, './src/hooks'),
        '@/layouts': resolve(__dirname, './src/layouts'),
        '@/pages': resolve(__dirname, './src/pages'),
        '@/services': resolve(__dirname, './src/services'),
        '@/types': resolve(__dirname, './src/types'),
        '@/utils': resolve(__dirname, './src/utils'),
        '@/stores': resolve(__dirname, './src/stores'),
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
