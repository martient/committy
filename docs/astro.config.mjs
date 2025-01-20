// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	redirects: {
		'/install': 'https://raw.githubusercontent.com/martient/committy/refs/heads/main/install.sh',
	},
	integrations: [
		starlight({
			title: 'Committy Docs',
			social: {
				github: 'https://github.com/martient/committy',
			},
			sidebar: [
				{
					label: 'Intro',
					items: [
						{ label: 'What is committy?', slug: "getting-started/what-is-committy" },
						{ label: 'Installation', slug: 'getting-started/installation' },
					],
				},
				{
					label: 'Reference',
					autogenerate: { directory: 'reference' },
				},
				{
					label: 'Project',
					autogenerate: { directory: 'project' },
				},
			],
			
		}),
	],
});
