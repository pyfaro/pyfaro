#!/usr/bin/env python3
"""Check the metadata of a built Pyfaro site (website/site).

Every page must carry a title, its own description, a canonical URL, the Open
Graph and Twitter card tags and an existing social preview image; the site must
ship a sitemap, a robots.txt that points to it and the favicons.
"""

import struct
import sys
from html.parser import HTMLParser
from pathlib import Path

SITE_URL = "https://pyfaro.dev/"
SITE_DESCRIPTION = "The unified Python development toolchain, built with Rust."
REQUIRED_PROPERTIES = (
    "og:type", "og:site_name", "og:title", "og:description", "og:url",
    "og:image", "og:image:width", "og:image:height", "og:image:alt",
)
REQUIRED_NAMES = (
    "twitter:card", "twitter:title", "twitter:description", "twitter:image",
)
IMAGE_MAX_BYTES = 1_000_000


class Head(HTMLParser):
    def __init__(self):
        super().__init__()
        self.title = ""
        self.meta = {}
        self.links = {}
        self._in_title = False

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == "title":
            self._in_title = True
        elif tag == "meta":
            key = attrs.get("property") or attrs.get("name")
            if key:
                self.meta[key] = attrs.get("content", "")
        elif tag == "link" and attrs.get("rel"):
            self.links.setdefault(attrs["rel"], []).append(attrs.get("href", ""))

    def handle_endtag(self, tag):
        if tag == "title":
            self._in_title = False

    def handle_data(self, data):
        if self._in_title:
            self.title += data


def png_size(path):
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    return struct.unpack(">II", data[16:24])


def check_page(site, page, errors):
    rel = page.relative_to(site).parent.as_posix()
    url = SITE_URL if rel == "." else f"{SITE_URL}{rel}/"
    head = Head()
    head.feed(page.read_text(encoding="utf-8"))

    def fail(message):
        errors.append(f"{page.relative_to(site)}: {message}")

    if not head.title.strip():
        fail("missing <title>")
    description = head.meta.get("description", "")
    if not description or description == SITE_DESCRIPTION:
        fail("missing page description (add `description:` front matter)")
    if head.links.get("canonical") != [url]:
        fail(f"canonical link is {head.links.get('canonical')}, expected [{url!r}]")
    for key in REQUIRED_PROPERTIES + REQUIRED_NAMES:
        if not head.meta.get(key):
            fail(f"missing <meta> {key}")
    if head.meta.get("og:url") != url:
        fail(f"og:url is {head.meta.get('og:url')!r}, expected {url!r}")
    if head.meta.get("og:description") != description:
        fail("og:description differs from the page description")
    if head.meta.get("twitter:card") != "summary_large_image":
        fail("twitter:card is not summary_large_image")
    image = head.meta.get("og:image", "")
    if image != head.meta.get("twitter:image"):
        fail("og:image and twitter:image differ")
    if not image.startswith(SITE_URL):
        fail(f"og:image {image!r} is not an absolute pyfaro.dev URL")
    return image


def main():
    site = Path(sys.argv[1] if len(sys.argv) > 1 else "website/site")
    errors = []
    pages = sorted(p for p in site.rglob("index.html"))
    if not pages:
        sys.exit(f"no pages found in {site}")
    images = {check_page(site, page, errors) for page in pages}

    for image in images:
        path = site / image.removeprefix(SITE_URL)
        if not path.is_file():
            errors.append(f"social preview {image} is not in the built site")
        elif path.stat().st_size >= IMAGE_MAX_BYTES:
            errors.append(f"{path} is {path.stat().st_size} bytes, over 1 MB")
        elif png_size(path) != (1280, 640):
            errors.append(f"{path} is not a 1280x640 PNG")

    sitemap = site / "sitemap.xml"
    if not sitemap.is_file():
        errors.append("sitemap.xml is missing")
    else:
        text = sitemap.read_text(encoding="utf-8")
        for page in pages:
            rel = page.relative_to(site).parent.as_posix()
            url = SITE_URL if rel == "." else f"{SITE_URL}{rel}/"
            if f"<loc>{url}</loc>" not in text:
                errors.append(f"sitemap.xml does not list {url}")
    robots = site / "robots.txt"
    if not robots.is_file() or f"Sitemap: {SITE_URL}sitemap.xml" not in robots.read_text():
        errors.append("robots.txt is missing or does not point to the sitemap")
    for icon in ("favicon.ico", "assets/favicon.svg", "assets/favicon.png",
                 "assets/apple-touch-icon.png"):
        if not (site / icon).is_file():
            errors.append(f"{icon} is missing")

    if errors:
        print("\n".join(errors), file=sys.stderr)
        sys.exit(f"{len(errors)} site metadata problem(s)")
    print(f"site metadata OK: {len(pages)} pages, {len(images)} social image(s)")


if __name__ == "__main__":
    main()
