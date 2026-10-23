#!/usr/bin/env python3
"""Import local display/text fonts from a user-supplied, cleartext EA Qt resource.

No client code is executed. Only the two known font resources are written.
"""
import argparse
import struct
import zlib
from pathlib import Path


def read_fonts(bundle):
    data = bundle.read_bytes()
    if len(data) < 20 or len(data) > 64 * 1024 * 1024:
        raise ValueError('Unexpected resource size')
    magic, version, tree, payloads, names = struct.unpack_from('>4sIIII', data)
    if magic != b'qres' or version not in (1, 2, 3):
        raise ValueError('Expected a cleartext Qt resource version 1, 2 or 3')
    stride = 14 if version == 1 else 22
    visited = set()
    result = {}
    wanted = {'Quantum/font_primary_regular.otf': 'ea-display.otf',
              'Quantum/font_secondary_regular.otf': 'ea-text.otf'}

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
            if size > 8 * 1024 * 1024 or start + size > len(data):
                raise ValueError('Invalid font payload')
            value = data[start:start + size]
            if flags & 4:
                raise ValueError('Zstandard-compressed fonts are not supported')
            if flags & 1:
                expected, = struct.unpack_from('>I', value)
                if expected > 8 * 1024 * 1024:
                    raise ValueError('Font too large')
                decoder = zlib.decompressobj()
                value = decoder.decompress(value[4:], expected + 1)
                if not decoder.eof or decoder.unconsumed_tail:
                    raise ValueError('Invalid compressed font stream')
                if len(value) != expected:
                    raise ValueError('Invalid compressed font length')
            if not value.startswith((b'OTTO', b'\x00\x01\x00\x00')):
                raise ValueError('Invalid OpenType font')
            result[wanted[path]] = value
    walk(0, '')
    if len(result) != 2:
        raise ValueError('EA Text/Display fonts were not found in this client version')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('ea_desktop', type=Path, help='Directory containing quantum.rcc, or quantum.rcc itself')
    parser.add_argument('output', type=Path, help='Destination ea-assets directory beside maxima')
    args = parser.parse_args()
    bundle = args.ea_desktop / 'quantum.rcc' if args.ea_desktop.is_dir() else args.ea_desktop
    fonts = read_fonts(bundle)
    args.output.mkdir(parents=True, exist_ok=True)
    for name, value in fonts.items():
        (args.output / name).write_bytes(value)
        print('Imported', name, len(value), 'bytes')
    print('These assets stay local. Do not commit the imported ea-assets directory.')


if __name__ == '__main__':
    main()
