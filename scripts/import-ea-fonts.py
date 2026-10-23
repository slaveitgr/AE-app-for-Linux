#!/usr/bin/env python3
"""Import local typography and navigation icons from cleartext EA Qt resources.

No client code is executed. Only known typography and navigation resources are written.
"""
import argparse
import re
import struct
import xml.etree.ElementTree as ET
import zlib
from pathlib import Path


FONT_RESOURCES = {
    'Quantum/font_primary_regular.otf': 'ea-display.otf',
    'Quantum/font_secondary_regular.otf': 'ea-text.otf',
    'Quantum/font_primary_bold.otf': 'ea-display-bold.otf',
    'Quantum/font_secondary_bold.otf': 'ea-text-bold.otf',
}
ICON_NAMES = (
    'HOME', 'BROWSE', 'COLLECTION', 'EA_PLAY_MEDALLION', 'DOWNLOAD_QUEUE',
    'Settings', 'Search', 'Heart', 'Notification', 'CHEVRON_LEFT',
    'CHEVRON_RIGHT', 'CHEVRON_DOWN', 'FRIENDS', 'Avatar', 'Offline',
)
ICON_RESOURCES = {
    f'Quantum/Assets/qml-icons/md/{name}.svg': f'icons/{name}.svg'
    for name in ICON_NAMES
}


def validate_font(value):
    if not value.startswith((b'OTTO', b'\x00\x01\x00\x00')):
        raise ValueError('Invalid OpenType font')


def validate_svg(value):
    upper = value.upper()
    if b'<!DOCTYPE' in upper or b'<!ENTITY' in upper or re.search(rb'<\?(?!xml(?:\s|>))', value, re.I):
        raise ValueError('SVG declarations and processing instructions are not supported')
    try:
        root = ET.fromstring(value)
    except ET.ParseError as error:
        raise ValueError('Invalid SVG icon') from error
    allowed_elements = {'svg', 'path', 'g', 'defs', 'clipPath', 'rect', 'circle',
                        'ellipse', 'line', 'polyline', 'polygon'}
    allowed_attributes = {'class', 'id', 'viewBox', 'width', 'height', 'd', 'x', 'y',
                          'x1', 'x2', 'y1', 'y2', 'cx', 'cy', 'r', 'rx', 'ry',
                          'points', 'transform', 'fill', 'stroke', 'stroke-width',
                          'stroke-linecap', 'stroke-linejoin', 'fill-rule', 'clip-rule',
                          'opacity', 'fill-opacity', 'stroke-opacity', 'clip-path'}
    if root.tag not in ('svg', '{http://www.w3.org/2000/svg}svg'):
        raise ValueError('Expected an SVG root element')
    for element in root.iter():
        name = element.tag.removeprefix('{http://www.w3.org/2000/svg}')
        if name not in allowed_elements:
            raise ValueError(f'Unsupported SVG element: {name}')
        for attribute, text in element.attrib.items():
            if attribute not in allowed_attributes:
                raise ValueError(f'Unsupported SVG attribute: {attribute}')
            if 'url(' in text.lower() and not re.fullmatch(r'url\(#[A-Za-z_][\w:.-]*\)', text):
                raise ValueError('External SVG references are not supported')


def read_resources(bundle, wanted, validate, maximum_payload):
    data = bundle.read_bytes()
    if len(data) < 20 or len(data) > 64 * 1024 * 1024:
        raise ValueError('Unexpected resource size')
    magic, version, tree, payloads, names = struct.unpack_from('>4sIIII', data)
    if magic != b'qres' or version not in (1, 2, 3):
        raise ValueError('Expected a cleartext Qt resource version 1, 2 or 3')
    stride = 14 if version == 1 else 22
    visited = set()
    result = {}

    def unpack(fmt, offset):
        if offset < 0 or offset + struct.calcsize(fmt) > len(data):
            raise ValueError('Resource offset out of bounds')
        return struct.unpack_from(fmt, data, offset)

    def walk(index, parent, depth=0):
        if index in visited or depth > 64:
            raise ValueError('Invalid resource tree')
        visited.add(index)
        offset = tree + index * stride
        name_offset, flags = unpack('>IH', offset)
        length, = unpack('>H', names + name_offset)
        start = names + name_offset + 6
        if start + length * 2 > len(data):
            raise ValueError('Invalid resource name')
        name = data[start:start + length * 2].decode('utf-16be') if index else ''
        if '/' in name or '\\' in name or name in ('.', '..'):
            raise ValueError('Unsafe resource name')
        path = '/'.join(filter(None, [parent, name]))
        if flags & 2:
            count, first = unpack('>II', offset + 6)
            if count > len(data) // stride:
                raise ValueError('Invalid child count')
            for child in range(first, first + count):
                walk(child, path, depth + 1)
        elif path in wanted:
            _, _, relative = unpack('>HHI', offset + 6)
            size, = unpack('>I', payloads + relative)
            start = payloads + relative + 4
            if size > maximum_payload or start + size > len(data):
                raise ValueError('Invalid resource payload')
            value = data[start:start + size]
            if flags & 4:
                raise ValueError('Zstandard-compressed resources are not supported')
            if flags & 1:
                expected, = struct.unpack_from('>I', value)
                if expected > maximum_payload:
                    raise ValueError('Resource too large')
                decoder = zlib.decompressobj()
                value = decoder.decompress(value[4:], expected + 1)
                if not decoder.eof or decoder.unconsumed_tail or decoder.unused_data:
                    raise ValueError('Invalid compressed resource stream')
                if len(value) != expected:
                    raise ValueError('Invalid compressed resource length')
            validate(value)
            result[wanted[path]] = value
    walk(0, '')
    if len(result) != len(wanted):
        missing = sorted(set(wanted.values()) - result.keys())
        raise ValueError('Known resources were not found in this client version: ' + ', '.join(missing))
    return result


def read_fonts(bundle):
    return read_resources(bundle, FONT_RESOURCES, validate_font, 8 * 1024 * 1024)


def read_icons(bundle):
    return read_resources(bundle, ICON_RESOURCES, validate_svg, 256 * 1024)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('ea_desktop', type=Path, help='EA Desktop directory, or quantum.rcc (icons use sibling assets.rcc)')
    parser.add_argument('output', type=Path, help='Destination ea-assets directory beside aeapp')
    args = parser.parse_args()
    bundle = args.ea_desktop / 'quantum.rcc' if args.ea_desktop.is_dir() else args.ea_desktop
    fonts = read_fonts(bundle)
    icons_bundle = bundle.with_name('assets.rcc')
    resources = {**fonts, **(read_icons(icons_bundle) if icons_bundle.is_file() else {})}
    args.output.mkdir(parents=True, exist_ok=True)
    for name, value in resources.items():
        destination = args.output / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(value)
        print('Imported', name, len(value), 'bytes')
    if not icons_bundle.is_file():
        print('Navigation icons were not imported: sibling assets.rcc was not found.')
    print('These assets stay local. Do not commit the imported ea-assets directory.')


if __name__ == '__main__':
    main()
