#!/usr/bin/env python3
"""Manually cache the anonymous official EA web catalog for native Browse.

Uses Python's standard library. No JavaScript, authentication, checkout, or
account endpoints are used. Imported artwork and metadata stay in the selected
local asset directory; running a preview never invokes this script.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
from html.parser import HTMLParser
import json
from pathlib import Path
import re
import sys
from urllib.parse import urljoin, urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener


SOURCE_URL = 'https://store.ea.com/commerce/app/products'
IMAGE_HOSTS = {'app-images.ea.com', 'media.contentapi.ea.com'}
VOID_TAGS = {'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input',
             'link', 'meta', 'param', 'source', 'track', 'wbr'}


def allowed_url(url, image=False):
    value = urlsplit(url)
    if value.scheme != 'https' or value.username or value.password:
        return False
    if value.port not in (None, 443) or value.fragment:
        return False
    if image:
        return value.hostname in IMAGE_HOSTS
    return (value.hostname == 'store.ea.com'
            and value.path.startswith('/commerce/app/')
            and not value.query)


class OfficialRedirects(HTTPRedirectHandler):
    def __init__(self, image):
        self.image = image
        super().__init__()

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        if not allowed_url(newurl, self.image):
            raise ValueError('Refusing a redirect outside the official allowlist')
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def fetch(url, image=False):
    if not allowed_url(url, image):
        raise ValueError('URL is outside the official catalog allowlist')
    request = Request(url, headers={
        'User-Agent': 'aeapp-public-catalog-import/1.0',
        'Accept': 'image/*' if image else 'text/html',
    })
    maximum = 8 * 1024 * 1024 if image else 3 * 1024 * 1024
    with build_opener(OfficialRedirects(image)).open(request, timeout=20) as response:
        if not allowed_url(response.url, image):
            raise ValueError('Response is outside the official allowlist')
        payload = response.read(maximum + 1)
        if len(payload) > maximum:
            raise ValueError('Response exceeds the bounded download size')
        return payload, response.headers.get_content_type()


class Node:
    def __init__(self, tag, attrs):
        self.tag = tag
        self.attrs = dict(attrs)
        self.children = []

    def has_class(self, name):
        return name in self.attrs.get('class', '').split()

    def find(self, name):
        if self.has_class(name):
            return self
        for child in self.children:
            if isinstance(child, Node):
                found = child.find(name)
                if found is not None:
                    return found
        return None

    def nodes(self):
        yield self
        for child in self.children:
            if isinstance(child, Node):
                yield from child.nodes()

    def text(self):
        return ' '.join(''.join(child.text() if isinstance(child, Node) else child
                                for child in self.children).split())


class CatalogParser(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.cards = []
        self.stack = []

    def handle_starttag(self, tag, attrs):
        node = Node(tag, attrs)
        if not self.stack:
            if tag != 'article' or not node.has_class('tile'):
                return
        else:
            self.stack[-1].children.append(node)
        if tag not in VOID_TAGS:
            self.stack.append(node)

    def handle_startendtag(self, tag, attrs):
        self.handle_starttag(tag, attrs)
        if tag not in VOID_TAGS:
            self.handle_endtag(tag)

    def handle_endtag(self, tag):
        if tag in VOID_TAGS:
            return
        for index in range(len(self.stack) - 1, -1, -1):
            if self.stack[index].tag == tag:
                if index == 0:
                    self.cards.append(self.stack[0])
                del self.stack[index:]
                break

    def handle_data(self, data):
        if self.stack:
            self.stack[-1].children.append(data)


def parse_products(html):
    parser = CatalogParser()
    parser.feed(html)
    products = {}
    for card in parser.cards:
        title, link, poster = (card.find(name) for name in
                               ('tile-title', 'tile-link', 'tile-poster'))
        if title is None or link is None or poster is None:
            continue
        url = urljoin(SOURCE_URL, link.attrs.get('href', ''))
        image_url = urljoin(SOURCE_URL, poster.attrs.get('src', ''))
        slug = urlsplit(url).path.removeprefix('/commerce/app/products/')
        if (not allowed_url(url) or not allowed_url(image_url, True)
                or not re.fullmatch(r'[a-z0-9][a-z0-9-]{0,159}', slug)):
            continue
        item = {'title': title.text(), 'slug': slug, 'url': url,
                'image_url': image_url}
        price = card.find('price')
        if price is not None and price.text():
            item['price_label'] = price.text()
            currency = re.match(r'^([A-Z]{3})\s+\d', price.text())
            if currency:
                item['currency'] = currency.group(1)
        meta = card.find('tile-meta')
        if meta is not None:
            for chip in meta.nodes():
                if chip.has_class('chip') and re.fullmatch(r'(19|20)\d{2}', chip.text()):
                    item['release_year'] = int(chip.text())
                    break
        products.setdefault(slug, item)
    return sorted(products.values(), key=lambda item:
                  (-item.get('release_year', 0), item['title'].casefold(), item['slug']))


def write_atomic(path, payload):
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.is_symlink():
        raise ValueError('Refusing to replace an asset symlink')
    temporary = path.with_name(path.name + '.tmp')
    if temporary.is_symlink():
        raise ValueError('Refusing an asset temporary symlink')
    temporary.write_bytes(payload)
    temporary.replace(path)


def download_image(item, directory):
    try:
        payload, media_type = fetch(item['image_url'], True)
        if payload.startswith(b'\xff\xd8\xff') and media_type in ('image/jpeg', 'image/jpg'):
            extension = '.jpg'
        elif payload.startswith(b'\x89PNG\r\n\x1a\n') and media_type == 'image/png':
            extension = '.png'
        elif (payload.startswith(b'RIFF') and payload[8:12] == b'WEBP'
              and media_type == 'image/webp'):
            extension = '.webp'
        else:
            raise ValueError('Expected JPEG, PNG, or WebP image data')
        relative = Path('images') / (item['slug'] + extension)
        write_atomic(directory / relative, payload)
        item['image_file'] = relative.as_posix()
        item['image_sha256'] = hashlib.sha256(payload).hexdigest()
        return None
    except (OSError, ValueError) as error:
        return f"{item['slug']}: {error}"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path, help='Local ea-assets/catalog directory')
    parser.add_argument('--limit', type=int, default=30, help='Maximum products (1–100)')
    parser.add_argument('--max-images', type=int, default=20, help='Maximum images (0–20)')
    parser.add_argument('--html', type=Path, help='Parse a previously saved official catalog HTML file')
    args = parser.parse_args()
    if not 1 <= args.limit <= 100 or not 0 <= args.max_images <= 20:
        parser.error('--limit must be 1–100 and --max-images must be 0–20')
    if args.html:
        payload = args.html.read_bytes()
        if len(payload) > 3 * 1024 * 1024:
            parser.error('Saved HTML exceeds the bounded source size')
    else:
        payload, media_type = fetch(SOURCE_URL)
        if media_type != 'text/html':
            raise ValueError('Official catalog did not return HTML')
    products = parse_products(payload.decode('utf-8'))[:args.limit]
    if not products:
        raise ValueError('No recognizable official product cards; catalog format may have changed')
    directory = args.output.expanduser().resolve()
    directory.mkdir(parents=True, exist_ok=True)
    with ThreadPoolExecutor(max_workers=4) as pool:
        failures = list(pool.map(lambda item: download_image(item, directory),
                                 products[:args.max_images]))
    for failure in failures:
        if failure:
            print('Image skipped: ' + failure, file=sys.stderr)
    result = {
        'version': 1,
        'source_url': SOURCE_URL,
        'fetched_at': datetime.now(timezone.utc).isoformat(timespec='seconds'),
        'source_sha256': hashlib.sha256(payload).hexdigest(),
        'price_context': 'Anonymous official page response; currency and offers may vary by region and time.',
        'products': products,
    }
    write_atomic(directory / 'products.json',
                 (json.dumps(result, ensure_ascii=False, indent=2) + '\n').encode('utf-8'))
    count = sum('image_file' in item for item in products)
    print(f'Imported {len(products)} official products and {count} images into {directory}')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError) as error:
        raise SystemExit(str(error)) from error
