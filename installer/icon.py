"""Draws the OpenECraft icon (a normally open contact on a blue tile).

Writes assets/icon.png (window icon, Linux packages), assets/icon.ico (Windows
installer, shortcuts, file association), assets/icon.icns (macOS app bundle) and
assets/icon.svg (scalable Linux icon). Pure Python, no dependencies:

    python installer/icon.py
"""

import math
import pathlib
import struct
import zlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
SIZES = [16, 24, 32, 48, 64, 128, 256]
# Entries of the macOS icon: (type, size); the @2x types reuse the double size.
ICNS = [(b"icp4", 16), (b"icp5", 32), (b"icp6", 64), (b"ic07", 128), (b"ic08", 256), (b"ic09", 512),
        (b"ic11", 32), (b"ic12", 64), (b"ic13", 256), (b"ic14", 512)]
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


def icns(pngs):
    body = b"".join(kind + struct.pack(">I", 8 + len(pngs[size])) + pngs[size] for kind, size in ICNS)
    return b"icns" + struct.pack(">I", 8 + len(body)) + body


def svg():
    hexa = "#{:02x}{:02x}{:02x}".format
    lines = "".join(
        f'<line x1="{a[0] * 256:g}" y1="{a[1] * 256:g}" x2="{b[0] * 256:g}" y2="{b[1] * 256:g}" stroke="{hexa(*c)}"/>'
        for a, b, c in STROKES
    )
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">'
        '<defs><linearGradient id="tile" gradientUnits="userSpaceOnUse" x1="0" y1="0" x2="0" y2="256">'
        f'<stop offset="0" stop-color="{hexa(*TOP)}"/><stop offset="1" stop-color="{hexa(*BOTTOM)}"/>'
        '</linearGradient></defs>'
        '<rect x="7.68" y="7.68" width="240.64" height="240.64" rx="51.2" fill="url(#tile)"/>'
        f'<g stroke-width="19.2" stroke-linecap="round">{lines}</g></svg>\n'
    )


def main():
    pngs = {size: png(size, render(size)) for size in sorted(set(SIZES) | {size for _, size in ICNS})}
    assets = ROOT / "assets"
    assets.mkdir(exist_ok=True)
    (assets / "icon.png").write_bytes(pngs[256])
    (assets / "icon.ico").write_bytes(ico([(size, pngs[size]) for size in SIZES]))
    (assets / "icon.icns").write_bytes(icns(pngs))
    (assets / "icon.svg").write_text(svg(), newline="\n")


if __name__ == "__main__":
    main()
