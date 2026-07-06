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
				src: './src/assets/ludi_logo.png',
				alt: 'Ludi logo',
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
