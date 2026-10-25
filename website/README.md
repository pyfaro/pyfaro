# Pyfaro documentation site

Source of the Pyfaro documentation, built with [MkDocs](https://www.mkdocs.org/)
and [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/).
The site is published at <https://pyfaro.dev/>.

## Preview and build

The tooling is a non-package [uv](https://docs.astral.sh/uv/) project pinned
in `website/pyproject.toml` and `website/uv.lock`. From the repository root:

```sh
# Live preview at http://127.0.0.1:8000/pyfaro/
uv run --project website --locked mkdocs serve -f website/mkdocs.yml

# Strict build, the same command CI runs (output in website/site/)
uv run --project website --locked mkdocs build --strict -f website/mkdocs.yml
```

The strict build must pass with no warnings (broken links, missing pages).

## Layout

| Path | Content |
| --- | --- |
| `website/mkdocs.yml` | Site config, theme features, extensions, **navigation** |
| `website/docs/index.md` | Landing page |
| `website/docs/getting-started/`, `commands/`, `workflows/`, `concepts/`, `reference/` | Sections, one file per page |
| `website/docs/stylesheets/extra.css` | Brand theme (palette, cards, buttons, hero) |
| `website/overrides/main.html` | Open Graph and Twitter card tags and the extra icon links, added to every page |
| `website/docs/assets/` | Logos and favicon derived from `assets/branding/pyfaro-logo.svg`, plus `social-preview.png` |
| `website/docs/robots.txt`, `website/docs/favicon.ico` | Copied to the site root by MkDocs (the `.ico` and `assets/apple-touch-icon.png` are rendered from `assets/favicon.svg`) |

## Style rules

- **Accuracy first.** Document only what is implemented. Planned features
  (`dev`, `build`, workspaces, tasks) are never described as available.
- **Real outputs.** Run every example with the built binary and the real uv in
  a disposable directory, and paste the actual output. Trim only
  machine-specific paths. If a guide and the binary disagree, the binary wins.
- Concise and task-oriented: command first, short paragraphs, admonitions for
  notes and warnings, and a "Next steps" list at the end of each page.
- English only, original text.

## Add a page

1. Create the Markdown file under `website/docs/` with an H1 title and a
   `description:` front matter line (one sentence, about 150 characters; quote
   it if it contains a colon). It becomes the search and social preview text.
2. Add it to `nav` in `website/mkdocs.yml`.
3. Run the strict build and fix any warning.
4. Run `python3 -I .github/scripts/check_site_metadata.py website/site`, the
   check the Docs workflow runs on the built site.

## Site metadata

- `site_url` in `mkdocs.yml` is `https://pyfaro.dev/`: it drives the canonical
  links, the `sitemap.xml` MkDocs generates and the absolute image URLs.
- `overrides/main.html` adds Open Graph and Twitter card tags to every page.
  Material's social plugin is not used: it needs Cairo and Pango on every
  build machine and renders one card per page, while one branded image is
  enough here.
- The social preview image is generated from
  [`assets/branding/social-preview.svg`](../assets/branding/social-preview.svg);
  see [`assets/branding/README.md`](../assets/branding/README.md) to
  regenerate it. The same PNG is the repository's social preview, which is a
  repository setting (Settings → General → Social preview) that a maintainer
  uploads by hand.

## Custom domain and HTTPS

The site is served by GitHub Pages at `https://pyfaro.dev/`. DNS at the
registrar:

| Name | Type | Value |
| --- | --- | --- |
| `pyfaro.dev` | `A` | `185.199.108.153`, `185.199.109.153`, `185.199.110.153`, `185.199.111.153` |
| `pyfaro.dev` | `AAAA` | `2606:50c0:8000::153`, `2606:50c0:8001::153`, `2606:50c0:8002::153`, `2606:50c0:8003::153` |
| `www.pyfaro.dev` | `CNAME` | `pyfaro.github.io` (without the repository name) |

With the apex as the custom domain (Settings → Pages), GitHub Pages redirects
`www.pyfaro.dev` to `https://pyfaro.dev/` once its certificate covers both
names. If `https://www.pyfaro.dev` fails with a certificate error although
the DNS records above are right, the certificate has not been re-issued for
`www`: remove the custom domain in Settings → Pages, save, add `pyfaro.dev`
again and wait for the DNS check to pass (up to an hour, sometimes longer;
GitHub documents up to 24 hours), then tick "Enforce HTTPS".

## Authoring helpers

Extensions enabled: admonitions, collapsible `???` blocks, tabs
(`=== "Title"`), Mermaid fences, code annotations, `attr_list`,
`md_in_html`, definition lists and Material icons (`:material-name:`).

- Cards: `<div class="grid cards" markdown>` around a list; each item is a card.
- Buttons: `[Text](link){ .md-button }` and `{ .md-button .md-button--primary }`
  (gold).
- Badge: `<span class="pf-badge">alpha</span>`, or `pf-badge pf-badge--blue`.
- Landing hero: `.pf-hero`, `.pf-hero__logo`, `.pf-hero__text`, `.pf-hero__lead`,
  `.pf-hero__actions`.
- Colors are CSS variables in `extra.css`: `--pf-anthracite`, `--pf-gold`,
  `--pf-gold-soft`, `--pf-gold-pale`, `--pf-blue`; Material tokens
  (`--md-typeset-a-color`, `--md-accent-fg-color`) are tuned per scheme.
