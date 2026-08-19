import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import { SvelteKitPWA } from '@vite-pwa/sveltekit';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit(),
		SvelteKitPWA({
			srcDir: 'src',
			scope: '/',
			base: '/',
			strategies: 'injectManifest',
			filename: 'service-worker.ts',
			injectRegister: false,
			manifest: {
				name: 'Eisen',
				short_name: 'Eisen',
				description: 'Eisenhower matrix task manager with encrypted sync',
				start_url: '/',
				scope: '/',
				display: 'standalone',
				background_color: '#ffffff',
				theme_color: '#0f766e',
				icons: [
					{ src: '/icon.svg', sizes: 'any', type: 'image/svg+xml' },
					{ src: '/icon-192x192.png', sizes: '192x192', type: 'image/png' },
					{
						src: '/icon-512x512.png',
						sizes: '512x512',
						type: 'image/png',
						purpose: 'any maskable'
					}
				]
			},
			injectManifest: {
				globPatterns: ['client/**/*.{js,css,html,ico,png,svg,webp,woff,woff2}']
			},
			devOptions: {
				enabled: true,
				type: 'module'
			}
		})
	]
});
