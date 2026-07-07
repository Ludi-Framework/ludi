import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'astro/config';

export default defineConfig({
	vite: {
		plugins: [tailwindcss()],
	},
	integrations: [
		starlight({
			title: 'Ludi',
			description: 'An Express-style web framework for Lua, powered by Rust.',
			logo: {
				// Decorative: the "Ludi" site title sits right next to it, so an
				// empty alt avoids the alt-text flash while the image loads.
				src: './src/assets/ludi_logo.png',
				alt: '',
			},
			favicon: '/favicon.png',
			social: [
				{
					icon: 'github',
					label: 'GitHub',
					href: 'https://github.com/Ludi-Framework/ludi',
				},
			],
			editLink: {
				baseUrl: 'https://github.com/Ludi-Framework/ludi/edit/main/website/',
			},
			customCss: ['./src/styles/global.css'],
			components: {
				SocialIcons: './src/components/SocialIcons.astro',
				Head: './src/components/Head.astro',
				Footer: './src/components/Footer.astro',
				Hero: './src/components/Hero.astro',
			},
			expressiveCode: {
				// Code windows stay dark in both site themes, per the design.
				themes: ['one-dark-pro'],
				styleOverrides: {
					borderRadius: '1.25rem',
					borderColor: '#263252',
					borderWidth: '1px',
					codeBackground: '#0d1224',
					frames: {
						editorBackground: '#0d1224',
						editorTabBarBackground: '#111a33',
						editorActiveTabBackground: '#0d1224',
						editorActiveTabForeground: '#d8e0ff',
						terminalBackground: '#0d1224',
						terminalTitlebarBackground: '#111a33',
						terminalTitlebarForeground: '#7482a9',
						terminalTitlebarBorderBottomColor: '#263252',
						shadowColor: 'rgba(15, 23, 42, 0.18)',
					},
				},
			},
			defaultLocale: 'root',
			locales: {
				root: { label: 'English', lang: 'en' },
				es: { label: 'Español', lang: 'es' },
				'pt-br': { label: 'Português (Brasil)', lang: 'pt-BR' },
			},
			sidebar: [
				{
					label: 'Getting Started',
					translations: { es: 'Primeros pasos', 'pt-BR': 'Começando' },
					items: [
						{ slug: 'getting-started/installation' },
						{ slug: 'getting-started/quick-start' },
					],
				},
				{
					label: 'Guides',
					translations: { es: 'Guías', 'pt-BR': 'Guias' },
					items: [
						{ slug: 'guides/routing' },
						{ slug: 'guides/middleware' },
						{ slug: 'guides/request' },
						{ slug: 'guides/response' },
						{ slug: 'guides/listening' },
						{ slug: 'guides/hot-reload' },
						{ slug: 'guides/build-binary' },
					],
				},
				{
					label: 'Advanced',
					translations: { es: 'Avanzado', 'pt-BR': 'Avançado' },
					items: [{ slug: 'advanced/architecture' }],
				},
			],
		}),
	],
});
