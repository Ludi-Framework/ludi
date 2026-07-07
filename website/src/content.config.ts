import { docsLoader } from '@astrojs/starlight/loaders';
import { docsSchema } from '@astrojs/starlight/schema';
import { defineCollection, z } from 'astro:content';

/**
 * Homepage-only frontmatter consumed by the custom Hero override
 * (src/components/Hero.astro): the pill badge above the title and the
 * capability chips rendered under the CTAs. Declared per locale in each
 * index.mdx so the copy stays translatable.
 */
const ludiHomeSchema = z
	.object({
		badge: z.string(),
		chips: z.array(z.object({ title: z.string(), detail: z.string() })),
	})
	.partial();

export const collections = {
	docs: defineCollection({
		loader: docsLoader(),
		schema: docsSchema({
			extend: z.object({ ludi: ludiHomeSchema.optional() }),
		}),
	}),
};
