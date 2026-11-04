#!/usr/bin/env python3
"""Renders Steps's drawn icon (assets/src/icon/*.svg) to every platform size.

Adapted from kabar-app main: scripts/assets/export_icons.py. Kabar starts from
painted PNG layers; Steps's mark is plain SVG, so this renders the SVG layers
first and then cuts the same Android, macOS and notification sizes.

    pip install cairosvg pillow
    python3 scripts/assets/export_icons.py

Writes assets/export/** (committed, so builds need neither tool) and, once
chunk 20's android/ module exists, copies the launcher icons into it as
ic_steps_launcher* (dx writes its own ic_launcher into the app module and app
resources win, so ours ship under another name; the manifest points at them).
"""
from __future__ import annotations

import io
import platform
import shutil
import subprocess
from pathlib import Path

import cairosvg
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "assets" / "src" / "icon"
OUT = ROOT / "assets" / "export"
ANDROID_RES = OUT / "android" / "res"
MACOS_DIR = OUT / "macos"
PREVIEW_PATH = OUT / "preview.png"
ANDROID_MODULE_RES = ROOT / "android" / "src" / "main" / "res"

MASTER = 1024
ADAPTIVE_SIZES = {
    "mipmap-mdpi": 108,
    "mipmap-hdpi": 162,
    "mipmap-xhdpi": 216,
    "mipmap-xxhdpi": 324,
    "mipmap-xxxhdpi": 432,
}
LEGACY_SIZES = {
    "mipmap-mdpi": 48,
    "mipmap-hdpi": 72,
    "mipmap-xhdpi": 96,
    "mipmap-xxhdpi": 144,
    "mipmap-xxxhdpi": 192,
}
NOTIFICATION_SIZES = {
    "drawable-mdpi": 24,
    "drawable-hdpi": 36,
    "drawable-xhdpi": 48,
    "drawable-xxhdpi": 72,
    "drawable-xxxhdpi": 96,
}
ICONSET = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
]
# Sizes Dioxus bundles for the desktop app ([bundle] icon in crates/app/Dioxus.toml).
DESKTOP_SIZES = [32, 128, 256, 512]


def render(name: str, size: int = MASTER) -> Image.Image:
    png = cairosvg.svg2png(url=str(SRC / name), output_width=size, output_height=size)
    return Image.open(io.BytesIO(png)).convert("RGBA")


def save(img: Image.Image, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    # No timestamps or text chunks, so re-running gives byte-identical files.
    img.save(path, optimize=True)


def resized(img: Image.Image, size: int) -> Image.Image:
    return img.resize((size, size), Image.LANCZOS)


def circular_mask(size: int) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).ellipse((0, 0, size - 1, size - 1), fill=255)
    return mask


def apply_mask(img: Image.Image, mask: Image.Image) -> Image.Image:
    out = img.copy()
    out.putalpha(mask)
    return out


def fit_to_box(img: Image.Image, size: int, padding: float) -> Image.Image:
    """Crop to the visible glyph and scale it to fill `size` minus `padding` on each side."""
    cropped = img.crop(img.getchannel("A").getbbox() or (0, 0, img.width, img.height))
    inner = size - 2 * round(size * padding)
    scale = inner / max(cropped.width, cropped.height)
    w, h = max(1, round(cropped.width * scale)), max(1, round(cropped.height * scale))
    out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    out.alpha_composite(cropped.resize((w, h), Image.LANCZOS), ((size - w) // 2, (size - h) // 2))
    return out


def make_macos_master(full: Image.Image) -> Image.Image:
    """Apple's grid: an 824 px rounded square centred on a 1024 canvas."""
    canvas = Image.new("RGBA", (MASTER, MASTER), (0, 0, 0, 0))
    body = 824
    mask = Image.new("L", (body, body), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, body - 1, body - 1), radius=185, fill=255)
    offset = (MASTER - body) // 2
    canvas.paste(resized(full, body), (offset, offset), mask)
    return canvas


def write_adaptive_xml(anydpi_dir: Path) -> None:
    anydpi_dir.mkdir(parents=True, exist_ok=True)
    xml = """<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@mipmap/ic_launcher_background" />
    <foreground android:drawable="@mipmap/ic_launcher_foreground" />
    <monochrome android:drawable="@mipmap/ic_launcher_monochrome" />
</adaptive-icon>
"""
    for name in ("ic_launcher.xml", "ic_launcher_round.xml"):
        (anydpi_dir / name).write_text(xml, encoding="utf-8")


def sync_android_module() -> None:
    """Copy the launcher and notification icons into android/ as `ic_steps_*`."""
    if not (ROOT / "android").is_dir():
        print("android/ does not exist yet (chunk 20); skipped copying the launcher icons into it")
        return
    for src in sorted(ANDROID_RES.glob("*/*")):
        dest = ANDROID_MODULE_RES / src.parent.name / src.name.replace("ic_launcher", "ic_steps_launcher")
        dest.parent.mkdir(parents=True, exist_ok=True)
        if src.suffix == ".xml":
            dest.write_text(src.read_text(encoding="utf-8").replace("ic_launcher", "ic_steps_launcher"), encoding="utf-8")
        else:
            shutil.copyfile(src, dest)


def build_preview(full: Image.Image, macos_master: Image.Image, notif: Image.Image) -> None:
    """One sheet to eyeball every mask and size (not used by any build)."""
    canvas = Image.new("RGBA", (1500, 420), (246, 247, 244, 255))
    base = resized(full, 260)
    squircle = Image.new("L", (260, 260), 0)
    ImageDraw.Draw(squircle).rounded_rectangle((0, 0, 259, 259), radius=58, fill=255)
    for x, icon in zip((40, 320, 600), (base, apply_mask(base, circular_mask(260)), apply_mask(base, squircle))):
        canvas.alpha_composite(icon, (x, 80))
    canvas.alpha_composite(resized(macos_master, 260), (900, 80))
    tile = Image.new("RGBA", (200, 200), (29, 36, 32, 255))
    white = Image.new("RGBA", notif.size, (255, 255, 255, 0))
    white.putalpha(notif.getchannel("A"))
    tile.alpha_composite(resized(white, 120), (40, 40))
    canvas.alpha_composite(tile, (1220, 110))
    save(canvas, PREVIEW_PATH)


def main() -> None:
    background = render("background.svg")
    foreground = render("foreground.svg")
    monochrome = render("monochrome.svg")
    full = render("full.svg")

    for dirname, px in ADAPTIVE_SIZES.items():
        d = ANDROID_RES / dirname
        save(resized(background, px), d / "ic_launcher_background.png")
        save(resized(foreground, px), d / "ic_launcher_foreground.png")
        save(resized(monochrome, px), d / "ic_launcher_monochrome.png")
    write_adaptive_xml(ANDROID_RES / "mipmap-anydpi-v26")

    for dirname, px in LEGACY_SIZES.items():
        d = ANDROID_RES / dirname
        square = resized(full, px)
        save(square, d / "ic_launcher.png")
        save(apply_mask(square, circular_mask(px)), d / "ic_launcher_round.png")

    # Notification glyph fills the 24 dp grid with 2 dp padding (Material spec).
    for dirname, px in NOTIFICATION_SIZES.items():
        save(fit_to_box(monochrome, px, 2 / 24), ANDROID_RES / dirname / "ic_stat_steps.png")

    sync_android_module()

    iconset = MACOS_DIR / "Steps.iconset"
    macos_master = make_macos_master(full)
    for filename, size in ICONSET:
        save(resized(macos_master, size), iconset / filename)
    if platform.system() == "Darwin":
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(MACOS_DIR / "Steps.icns")], check=True)

    for size in DESKTOP_SIZES:
        save(resized(macos_master, size), OUT / "desktop" / f"{size}x{size}.png")

    build_preview(full, macos_master, fit_to_box(monochrome, 180, 2 / 24))
    print(f"wrote {OUT.relative_to(ROOT)}/")


if __name__ == "__main__":
    main()
