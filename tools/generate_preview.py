"""Make the picture shown when the link of the site is shared (WhatsApp, Telegram, Slack, social networks): mzlab/web/anteprima.png, 1200 x 630: the logo «mz» followed by the rest of the name («Lab»), one word.

The name is read from mzlab/web/appname.js (the only place where it is written). Only Pillow is needed (the logo is mzlab/web/logo.png).
WhatsApp shows the large preview only if the picture is below ~300 KB: the script checks it."""
import re
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "mzlab" / "web"
NAME = re.search(r'APP_NAME\s*=\s*"([^"]+)"', (WEB / "appname.js").read_text(encoding="utf-8")).group(1)
W, H, S = 1200, 630, 2                                    # drawn at double size, then reduced (smooth edges)
BOLD = next((p for p in ["/System/Library/Fonts/Supplemental/Arial Bold.ttf", "/System/Library/Fonts/Helvetica.ttc", "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf"] if Path(p).exists()), "Arial")
font = lambda p, n: ImageFont.truetype(p, int(n * S))


# where the letters of the logo are, in units of its side (measured on logo.png, 256 px): top of the lowercase m and z (x-height),
# their baseline, and how far the lightning reaches above and below
LOGO_XTOP, LOGO_BASE, LOGO_TOP, LOGO_BOT = 74 / 256, 172 / 256, 38 / 256, 218 / 256


def make():
    """The logo «mz» (with the lightning) followed by the rest of the name in text, read as ONE word: «Lab» on the same baseline as the m
    and the z of the logo, its lowercase letters as tall as theirs, close to the z; centred, with wide margins, on the light colour of the page."""
    rest = NAME[2:] if NAME.lower().startswith("mz") else NAME          # the logo already says «mz»: only the rest is written
    im = Image.new("RGB", (W * S, H * S), (247, 248, 250)); d = ImageDraw.Draw(im)
    L = 330 * S                                                          # side of the logo
    xh = L * (LOGO_BASE - LOGO_XTOP)                                     # x-height of the logo letters
    probe = font(BOLD, 1000 / S); b = d.textbbox((0, 0), "x", font=probe, anchor="ls")
    f = font(BOLD, 1000 / S * xh / (b[3] - b[1]))                       # font size whose x-height is that of the logo
    tb = d.textbbox((0, 0), rest, font=f, anchor="ls")                   # box of the text around its origin on the baseline
    gap = 0.06 * xh
    total = L + gap + (tb[2] - tb[0])
    x0 = (W * S - total) / 2
    top, bot = min(L * LOGO_TOP, L * LOGO_BASE + tb[1]), max(L * LOGO_BOT, L * LOGO_BASE + tb[3])      # whole drawing, from the logo top
    y0 = (H * S - (bot - top)) / 2 - top
    logo = Image.open(WEB / "logo.png").convert("RGBA").resize((int(L), int(L)), Image.LANCZOS)
    im.paste(logo, (int(round(x0)), int(round(y0))), logo)
    d.text((x0 + L + gap - tb[0], y0 + L * LOGO_BASE), rest, font=f, fill=(35, 38, 45), anchor="ls")
    return im.resize((W, H), Image.LANCZOS)


if __name__ == "__main__":
    out = WEB / "anteprima.png"
    make().convert("P", palette=Image.ADAPTIVE, colors=128).save(out, optimize=True)
    kb = out.stat().st_size / 1024
    print(f"{out}: {W}x{H}, {kb:.0f} KB")
    assert kb < 280, "too heavy for the WhatsApp large preview (~300 KB)"
