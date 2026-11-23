import { sveltekit } from '@sveltejs/kit/vite';
import adapter from '@sveltejs/adapter-static';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

// During `npm run dev`, API calls go to a Tuffie server on :7777.
const backend = process.env.TUFFIE_BACKEND ?? 'http://127.0.0.1:7777';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			// Single-page app: the Rust binary serves index.html for every route.
			adapter: adapter({ pages: 'build', assets: 'build', fallback: 'index.html', strict: false })
		})
	],
	server: {
		// The user guide lives in ../docs/guide and is bundled into the app.
		fs: { allow: ['..'] },
		proxy: {
			'/api': { target: backend, ws: true, changeOrigin: true }
		}
	},
	build: { chunkSizeWarningLimit: 900 }
});
