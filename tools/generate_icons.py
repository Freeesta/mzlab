"""Make the logo and every icon from the picture of the m/z logo (tools/logo_sorgente.jpg).

Writes into mzlab/web/: logo.png (transparent), logo.svg / favicon.svg / app-icon.svg (the PNG embedded, so they work as
<img> and as icons), favicon-32.png, favicon.ico, apple-touch-icon.png, app-icon-192/512.png, app-icon-maskable-512.png (site and PWA icons; the PWA ones and the apple-touch-icon carry the «web» badge). Only needs Pillow, scipy and numpy. From the project root:  python3 tools/generate_icons.py
"""
import base64, io
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage as ndi

ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "mzlab" / "web"
SRC = Path(__file__).parent / "logo_sorgente.jpg"


def cutout() -> Image.Image:
    """The picture without its white background (only the white connected to the border becomes transparent) and cropped to the content."""
    im = Image.open(SRC).convert("RGB")
    a = np.asarray(im).astype(np.int16)
    white = (a.min(axis=2) > 238)
    lab, _ = ndi.label(white)
    border = set(np.unique(np.concatenate([lab[0], lab[-1], lab[:, 0], lab[:, -1]]))) - {0}
    bg = np.isin(lab, list(border))
    alpha = ndi.gaussian_filter((~bg).astype(float), 1.1)
    alpha = np.clip((alpha - 0.15) / 0.7, 0, 1)
    rgba = np.dstack([a.astype(np.uint8), (alpha * 255).astype(np.uint8)])
    out = Image.fromarray(rgba, "RGBA")
    box = out.getchannel("A").point(lambda v: 255 if v > 20 else 0).getbbox()
    out = out.crop(box)
    s = max(out.size)
    sq = Image.new("RGBA", (s, s), (0, 0, 0, 0)); sq.paste(out, ((s - out.width) // 2, (s - out.height) // 2))
    return sq


def png(im: Image.Image) -> bytes:
    b = io.BytesIO(); im.save(b, "PNG", optimize=True); return b.getvalue()


def svg_with(im: Image.Image, size: int, background: bool = False) -> str:
    b64 = base64.b64encode(png(im.resize((size, size), Image.LANCZOS))).decode()
    bg = '<rect width="100" height="100" rx="22" fill="#ffffff"/>' if background else ""
    inner = (12, 76) if background else (0, 100)
    return (f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 100 100">{bg}'
            f'<image x="{inner[0]}" y="{inner[0]}" width="{inner[1]}" height="{inner[1]}" xlink:href="data:image/png;base64,{b64}"/></svg>\n')


def app_icon(core: Image.Image, size: int, rounded: bool = True) -> Image.Image:
    """White rounded square with the picture inside (macOS style margins)."""
    S = size * 4
    bg = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(bg).rounded_rectangle((0, 0, S - 1, S - 1), radius=int(S * 0.22) if rounded else 0, fill=(255, 255, 255, 255))
    m = int(S * 0.13); inner = core.resize((S - 2 * m, S - 2 * m), Image.LANCZOS)
    bg.alpha_composite(inner, (m, m))
    return bg.resize((size, size), Image.LANCZOS)


def web_badge(im: Image.Image, rounded: bool = True) -> Image.Image:
    """The icon of the installed web app (PWA «mzLab Web»): the same icon with a small «web» badge at the bottom right, so it differs from the desktop app's.
    The badge stays inside the maskable safe zone (inner 80%)."""
    S = im.width * 4
    big = im.convert("RGBA").resize((S, S), Image.LANCZOS)
    d = ImageDraw.Draw(big)
    f = ImageFont.load_default(size=int(S * 0.15))
    tw = d.textlength("web", font=f)
    px, py = int(S * 0.045), int(S * 0.025)
    w, h = int(tw + 2 * px), int(S * 0.15 + 2 * py)
    x1 = int(S * (0.86 if rounded else 0.82)); y1 = int(S * (0.86 if rounded else 0.82))
    box = (x1 - w, y1 - h, x1, y1)
    d.rounded_rectangle(box, radius=h // 2, fill=(30, 90, 170, 255))
    d.text(((box[0] + box[2]) / 2, (box[1] + box[3]) / 2), "web", font=f, fill=(255, 255, 255, 255), anchor="mm")
    return big.resize((im.width, im.height), Image.LANCZOS)


def main():
    core = cutout()
    (WEB / "logo.png").write_bytes(png(core.resize((256, 256), Image.LANCZOS)))
    (WEB / "logo.svg").write_text(svg_with(core, 128), encoding="utf-8")
    (WEB / "favicon.svg").write_text(svg_with(core, 64), encoding="utf-8")
    (WEB / "app-icon.svg").write_text(svg_with(core, 160, True), encoding="utf-8")
    (WEB / "favicon-32.png").write_bytes(png(core.resize((32, 32), Image.LANCZOS)))
    (WEB / "logo-64.png").write_bytes(png(core.resize((64, 64), Image.LANCZOS)))   # headers (28-32 px on screen): 6 kB instead of 110
    core.resize((256, 256), Image.LANCZOS).save(WEB / "favicon.ico", sizes=[(16, 16), (32, 32), (48, 48), (64, 64)])
    (WEB / "apple-touch-icon.png").write_bytes(png(web_badge(app_icon(core, 180, rounded=False), False).convert("RGB")))
    big = {}
    for s in (32, 64, 128, 192, 256, 512, 1024):
        big[s] = png(app_icon(core, s))
        if s in (192, 512):
            (WEB / f"app-icon-{s}.png").write_bytes(png(web_badge(app_icon(core, s))))
    (WEB / "app-icon-maskable-512.png").write_bytes(png(web_badge(app_icon(core, 512, rounded=False), False).convert("RGB")))
    print("icone scritte in", WEB)


if __name__ == "__main__":
    main()
