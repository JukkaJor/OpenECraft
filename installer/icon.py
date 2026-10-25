"""Draws the OpenECraft icon (a normally open contact on a blue tile).

Writes assets/icon.png (window icon) and assets/icon.ico (Windows installer,
shortcuts, file association). Pure Python, no dependencies:

    python installer/icon.py
"""

import math
import pathlib
import struct
import zlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
SIZES = [16, 24, 32, 48, 64, 128, 256]
SAMPLES = 4

TOP = (0x2C, 0x72, 0xCC)
BOTTOM = (0x15, 0x45, 0x86)
WHITE = (0xFF, 0xFF, 0xFF)
AMBER = (0xF5, 0xB8, 0x2E)

# Strokes in unit coordinates: (from, to, colour).
STROKES = [
    ((0.50, 0.14), (0.50, 0.38), WHITE),
    ((0.50, 0.38), (0.39, 0.38), WHITE),
    ((0.50, 0.62), (0.50, 0.86), WHITE),
    ((0.50, 0.62), (0.31, 0.33), AMBER),
]


def segment_distance(p, a, b):
    ax, ay = p[0] - a[0], p[1] - a[1]
    bx, by = b[0] - a[0], b[1] - a[1]
    t = max(0.0, min(1.0, (ax * bx + ay * by) / (bx * bx + by * by)))
    return math.hypot(ax - t * bx, ay - t * by)


def in_tile(x, y, inset=0.03, radius=0.2):
    lo, hi = inset + radius, 1 - inset - radius
    dx = max(lo - x, 0.0, x - hi)
    dy = max(lo - y, 0.0, y - hi)
    return dx * dx + dy * dy <= radius * radius and inset <= x <= 1 - inset and inset <= y <= 1 - inset


def colour_at(x, y, width):
    if not in_tile(x, y):
        return None
    colour = tuple(round(t + (b - t) * y) for t, b in zip(TOP, BOTTOM))
    for a, b, stroke in STROKES:
        if segment_distance((x, y), a, b) <= width / 2:
            colour = stroke
    return colour


def render(size):
    # Strokes stay at least 1.6 pixels wide in the small sizes.
    width = max(0.075, 1.6 / size)
    rows = []
    for py in range(size):
        row = bytearray()
        for px in range(size):
            r = g = b = a = 0
            for sy in range(SAMPLES):
                for sx in range(SAMPLES):
                    c = colour_at((px + (sx + 0.5) / SAMPLES) / size, (py + (sy + 0.5) / SAMPLES) / size, width)
                    if c:
                        r, g, b, a = r + c[0], g + c[1], b + c[2], a + 1
            row += bytes((r // a, g // a, b // a, a * 255 // SAMPLES**2) if a else (0, 0, 0, 0))
        rows.append(bytes(row))
    return rows


def png(size, rows):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    raw = b"".join(b"\0" + row for row in rows)
    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def ico(images):
    out = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    for size, data in images:
        out += struct.pack("<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
    return out + b"".join(data for _, data in images)


def main():
    images = [(size, png(size, render(size))) for size in SIZES]
    assets = ROOT / "assets"
    assets.mkdir(exist_ok=True)
    (assets / "icon.png").write_bytes(images[-1][1])
    (assets / "icon.ico").write_bytes(ico(images))


if __name__ == "__main__":
    main()
