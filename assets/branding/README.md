# Pyfaro branding assets

[`pyfaro-logo.svg`](pyfaro-logo.svg) is the editable vector master of the
Pyfaro logo. Its `1000 × 1000` viewBox preserves the original composition
and margins, with an opaque white background.

## Editing

The SVG contains named groups and individually editable shapes:

- **Background:** a separate white rectangle.
- **Symbol:** the P monogram, golden light beam, lighthouse, and blue ribbon.
- **Lighthouse:** separate roof, lantern chamber, window, gallery, tower,
  and curved base.
- **Wordmark:** one vector path per letter in “Pyfaro”.

The main groups are labeled as layers for Inkscape. All lettering is
outlined; the logo requires no external fonts or embedded raster images.
Edit this SVG as the source of truth and generate raster exports from it
when needed.

## Social preview

[`social-preview.svg`](social-preview.svg) is the editable source of the
`1280 × 640` social preview image: the repository's preview on GitHub and the
Open Graph / Twitter card image of every site page. It embeds the logo and
sets the tagline in [Inter](https://rsms.me/inter/) (install it first, or the
text falls back to another sans-serif font). Regenerate the PNG the site
serves, which must stay under 1 MB, with:

```sh
rsvg-convert -w 1280 -h 640 assets/branding/social-preview.svg \
  -o website/docs/assets/social-preview.png
```

The same PNG is the file to upload under Settings → General → Social preview
in the GitHub repository.
