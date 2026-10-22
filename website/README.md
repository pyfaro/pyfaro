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
| `website/docs/assets/` | Logos and favicon derived from `assets/branding/pyfaro-logo.svg` |

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

1. Create the Markdown file under `website/docs/` with an H1 title.
2. Add it to `nav` in `website/mkdocs.yml`.
3. Run the strict build and fix any warning.

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
