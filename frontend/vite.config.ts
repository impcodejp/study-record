import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

/**
 * Vite の設定。
 *
 * 開発時は `/api` への要求を Rust の API サーバー（既定 127.0.0.1:8080）へ転送する。
 * 本番では nginx が同じ役割を担う（deploy/nginx/study-record-http.conf を参照）。
 */
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: process.env.API_ORIGIN ?? 'http://127.0.0.1:8080',
        changeOrigin: false,
      },
    },
  },
})
