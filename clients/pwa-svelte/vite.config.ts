import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { SvelteKitPWA } from '@vite-pwa/sveltekit';

export default defineConfig({
	plugins: [
		sveltekit(),
		SvelteKitPWA({
			manifest: {
				name: 'Eisen',
				short_name: 'Eisen',
				start_url: '/',
				display: 'standalone',
				background_color: '#ffffff',
				theme_color: '#0f766e',
				icons: [
					{ src: '/icon.svg', sizes: 'any', type: 'image/svg+xml' },
					{ src: '/icon-192x192.png', sizes: '192x192', type: 'image/png' },
					{ src: '/icon-512x512.png', sizes: '512x512', type: 'image/png' }
				]
			},
			workbox: {
				globPatterns: ['client/**/*.{js,css,html,ico,png,svg,webp,woff,woff2}'],
				maximumFileSizeToCacheInBytes: 2 * 1024 * 1024
			}
		})
	]
});
