# Ludi Documentation Website

The documentation site for [Ludi](https://github.com/Ludi-Framework/ludi),
built with [Astro](https://astro.build) + [Starlight](https://starlight.astro.build)
and [Tailwind CSS v4](https://tailwindcss.com).

> This folder lives inside the framework repository but is a self-contained
> Node project. It is **never** packaged into the LuaRocks rock: the rockspec
> `build.install` section lists installed files explicitly, and prebuilt
> binary rocks don't touch the repository at all.

## Stack

- **Astro 7** with the **Starlight** docs theme (TypeScript config)
- **Tailwind CSS v4** via `@tailwindcss/vite` + `@astrojs/starlight-tailwind`
- **MDX** for all content pages (Starlight components: `Tabs`, `Steps`,
  `Card`, `Aside`)
- **Prettier** with `prettier-plugin-astro`,
  `prettier-plugin-organize-imports` and `prettier-plugin-tailwindcss`
- **pnpm** as the package manager

## Getting started

```bash
pnpm install
pnpm dev        # http://localhost:4321
```

## Commands

| Command             | Action                                   |
| :------------------ | :--------------------------------------- |
| `pnpm install`      | Install dependencies                     |
| `pnpm dev`          | Start the dev server at `localhost:4321` |
| `pnpm build`        | Build the production site to `./dist/`   |
| `pnpm preview`      | Preview the production build locally     |
| `pnpm format`       | Format the codebase with Prettier        |
| `pnpm format:check` | Check formatting without writing         |

## Project structure

```
website/
├── public/                  # Static assets (favicon)
├── src/
│   ├── assets/              # Images processed by Astro (logo)
│   ├── components/          # Astro components
│   │   ├── BuyMeACoffee.astro   # Header donation button (localized label)
│   │   ├── Footer.astro         # Starlight Footer override: logo + links
│   │   ├── Head.astro           # Starlight Head override + locale detection
│   │   └── SocialIcons.astro    # Starlight override: BMC button + GitHub
│   ├── content/
│   │   └── docs/            # MDX content — one folder per locale
│   │       ├── ...          # English (default, served at /)
│   │       ├── es/          # Spanish (served at /es/)
│   │       └── pt-br/       # Portuguese (served at /pt-br/)
│   ├── styles/
│   │   └── global.css       # Tailwind v4 theme + Ludi palette (light/dark)
│   └── content.config.ts    # Starlight content collection
├── astro.config.ts          # Site config: locales, sidebar, overrides
├── Dockerfile               # Multi-stage build → nginx static image
├── nginx.conf               # nginx config used by the Docker image
└── tsconfig.json
```

## Internationalization

Three locales, configured in `astro.config.ts`:

| Locale      | Path      | Label              |
| :---------- | :-------- | :----------------- |
| `en` (root) | `/`       | English            |
| `es`        | `/es/`    | Español            |
| `pt-BR`     | `/pt-br/` | Português (Brasil) |

Every page exists in all three languages, mirrored across
`src/content/docs/`, `src/content/docs/es/` and `src/content/docs/pt-br/`.
Sidebar group labels are translated via `translations` in `astro.config.ts`;
page labels come from each page's `title` frontmatter.

**Language detection** (`src/components/Head.astro`): on the first visit to
the site root, a small client script redirects to the locale matching the
browser's `navigator.languages`. Picking a language in the header persists
the choice to `localStorage` and wins over detection on later visits. Deep
links are never redirected.

## Theming

Starlight ships light/dark mode with a header toggle out of the box. The
Ludi palette (purple accent from the logo) is defined once in
`src/styles/global.css` as Tailwind `@theme` color ramps; the
`@astrojs/starlight-tailwind` integration maps them onto both themes.

Layout is **mobile first**: base styles target phones, `min-width` media
queries and responsive Tailwind variants (`lg:`) layer desktop behavior on
top.

## Buy me a coffee

The header button is `src/components/BuyMeACoffee.astro`. Update `BMC_URL`
there to point at the right Buy Me a Coffee account. The label is localized
per locale and collapses to just the ☕ icon on small screens.

## Docker deploy

Multi-stage image: pnpm build in `node:22-alpine`, served by `nginx:alpine`.

```bash
docker build -t ludi-docs .
docker run --rm -p 8080:80 ludi-docs
# http://localhost:8080
```

`nginx.conf` handles Astro's pretty URLs, the 404 page, gzip and immutable
caching for hashed assets under `/_astro/`.

## Adding a page

1. Create the MDX file in `src/content/docs/<section>/<slug>.mdx` (English).
2. Mirror it in `es/<section>/<slug>.mdx` and `pt-br/<section>/<slug>.mdx`.
3. Add the slug to the `sidebar` in `astro.config.ts` (one entry covers all
   locales).
4. Run `pnpm format` before committing.
