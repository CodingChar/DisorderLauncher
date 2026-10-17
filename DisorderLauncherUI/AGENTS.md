# DisorderLauncherUI

**Svelte 5 + Vite 8** visual mockup running inside Figma Make.

> This project used to be React. It was ported to Svelte 5 (runes: `$state`,
> `$derived`, `$props`) to match the stack of the real app. Keep it Svelte.

## Purpose

**This repository is only a visual mockup (prototipo visual) of the real project.** It exists to design, iterate on, and review the interface. It is not the shipping application.

- Real project: `C:\Users\Pollito\Desktop\developments\DisorderLaucher` (note: spelled "Laucher", one `n`) — a Tauri 2 + Svelte 5 app under `apps/desktop`
- This repo: `C:\Users\Pollito\Desktop\developments\DisorderLauncherUI`

Keep that split in mind when working here:

- **Don't** port real business logic, persistence, backend calls, or auth into this repo. Mock data is intentional and preferred.
- Placeholder content, simulated logs, and hardcoded client state are expected and correct, not oversights to fix.
- UI copy is in Spanish and should stay in Spanish unless asked otherwise.
- This repo is **not a git repository**. There is no diff, history, or branch to review against; "review the changes" means reviewing the current working tree.
- Porting a UI decision from here into `DisorderLaucher` is separate work, done there.

## Dependencies

- Runtime: **Svelte 5** (`svelte`)
- Build tooling: Vite 8, `@sveltejs/vite-plugin-svelte` 7, `svelte-check`, TypeScript 5.7, and `@tailwindcss/vite` for Tailwind CSS v4
- Formatting: oxfmt

## Development Server

A Vite development server is **already running** on `$PORT` (default 8443). You don't need to start it manually.

- Preview URL: The user can access the running app through the preview panel
- Hot reload: Changes to source files are reflected immediately

## Project Structure

This is the canonical project structure. Start with task-relevant files below. Only follow imports or inspect other files when required, when a documented path is missing, or when the repository contradicts this guide.

- `src/main.ts` - Svelte entrypoint; imports `src/index.css` and mounts `src/App.svelte` into `#root`
- `src/App.svelte` - Primary application component and the usual starting point for UI work
- `src/lib/types.ts` - Shared domain types and mock data lists (`Instance`, `Session`, `Skin`, `Profile`, `defaults`)
- `src/lib/Icon.svelte` - Icon set rendered as SVG `currentColor` parts
- `src/lib/ItemLogo.svelte` - Instance icon: CSS `.block` div for blocks, SVG for items
- `src/lib/Avatar.svelte` - Pixel-art avatar; declares a `<script>` only so its type is exported
- `src/lib/SkinPreview.svelte` - CSS-drawn skin preview driven by custom properties
- `src/lib/InstanceEditor.svelte` - Instance form, `<details>` advanced options, icon `<dialog>`
- `src/lib/SessionMonitor.svelte` - Running-session card plus simulated log console
- `src/index.css` - Global CSS entrypoint and Tailwind CSS v4 import
- `index.html` - Vite HTML shell containing the `#root` element and loading `src/main.ts`
- `svelte.config.js` - Svelte config (used by `vite-plugin-svelte` and `svelte-check`)
- `scripts/prune-css.mjs` - one-off maintenance helper: deletes `src/index.css` rules whose selector references a class that no longer appears in any `.svelte` file. It parses at rule level (the CSS is minified, several rules per line) and understands `@media` blocks and comma-separated selector groups. Review its output list before keeping it.
- `package.json` - Project dependencies and the Vite build, development, preview, check, prune, and formatting scripts
- `vite.config.ts` - Vite config with Svelte, Tailwind CSS v4, and Figma Make plugins plus the `@` alias for `src`
- `.mise.toml` - Toolchain versions for Node.js and pnpm

## Checks

- `pnpm check` - `svelte-check` (types + a11y + Svelte compiler warnings). **`tsc --noEmit` no longer covers `.svelte` files**, so this is the type gate.
- `pnpm build` - production build
- `pnpm prune:css` - removes dead CSS rules (see `scripts/prune-css.mjs`). Not part of any build step; run it deliberately.

## Styling

This project uses **Tailwind CSS v4** through the `@tailwindcss/vite` plugin configured in `vite.config.ts`. `src/index.css` imports Tailwind with `@import 'tailwindcss';`. Use Tailwind utility classes directly in JSX/Svelte markup and put global CSS or Tailwind v4 theme customization in `src/index.css`. This scaffold does not need a Tailwind config file or PostCSS config.

`src/main.ts` imports `src/index.css`, so global font wiring belongs in `src/index.css`. Keep CSS `@import` statements first, then add any `@font-face` rules and font-family defaults there.

`src/index.css` holds the whole design system in minified one-line-per-block form, followed by appended responsive sections (`max-height` breakpoints first, then narrower `max-width` breakpoints). When adding a media query inside one of those blocks, match the specificity of the rules it must override — e.g. `.sidebar nav`, not `nav`, or the ≤720px width block wins regardless of source order.

## Svelte gotchas hit in this project

- Attribute values are expressions: `pattern="[A-Za-z0-9_]{1,16}"` parses `{1,16}` as a comma operator and silently breaks validation. Write `pattern={'[A-Za-z0-9_]{1,16}'}`.
- Keyed `{#each}` keys must be unique. `Icon.svelte` uses the index because several icons contain multiple `<rect>`/`<circle>` parts that collide on any field-derived key.
- The `invalid` event does not bubble. Attach it with a capture-phase action (`revealInvalidFields` in `InstanceEditor.svelte`) instead of an `oninvalid` attribute.
- Avoid the `autofocus` attribute; use the `autofocus` action, which Svelte does not lint.
- Write `.svelte` files with the editor tooling, not PowerShell `Set-Content`: it corrupts UTF-8 (Spanish accents) and naive regexes can rewrite `<path />` into `<path></p>`.
