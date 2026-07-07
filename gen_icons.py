"""生成 Tauri 所需的应用图标（占位图标，之后可用 `npm run tauri icon 自己的图.png` 替换）。"""
import os
from PIL import Image, ImageDraw, ImageFont

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "src-tauri", "icons")


def make_base(size):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    # 圆角渐变背景（紫->蓝）
    for y in range(size):
        t = y / size
        r = int(99 + (56 - 99) * t)
        g = int(102 + (189 - 102) * t)
        b = int(241 + (248 - 241) * t)
        d.line([(0, y), (size, y)], fill=(r, g, b, 255))
    # 圆角遮罩
    mask = Image.new("L", (size, size), 0)
    md = ImageDraw.Draw(mask)
    radius = int(size * 0.22)
    md.rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=255)
    img.putalpha(mask)
    # 画一个 "译" 字
    d = ImageDraw.Draw(img)
    txt = "译"
    fsize = int(size * 0.6)
    font = None
    for path in [
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc",
        "/mnt/c/Windows/Fonts/msyh.ttc",
        "/mnt/c/Windows/Fonts/simhei.ttf",
    ]:
        try:
            font = ImageFont.truetype(path, fsize)
            break
        except Exception:
            continue
    if font is None:
        font = ImageFont.load_default()
        txt = "A"
    bbox = d.textbbox((0, 0), txt, font=font)
    tw, th = bbox[2] - bbox[0], bbox[3] - bbox[1]
    d.text(((size - tw) / 2 - bbox[0], (size - th) / 2 - bbox[1]), txt,
           font=font, fill=(255, 255, 255, 255))
    return img


os.makedirs(OUT, exist_ok=True)
base = make_base(512)
base.resize((32, 32), Image.LANCZOS).save(os.path.join(OUT, "32x32.png"))
base.resize((128, 128), Image.LANCZOS).save(os.path.join(OUT, "128x128.png"))
base.resize((256, 256), Image.LANCZOS).save(os.path.join(OUT, "128x128@2x.png"))
base.save(os.path.join(OUT, "icon.png"))
# Windows .ico（多尺寸）
base.save(os.path.join(OUT, "icon.ico"),
          sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
print("icons generated in", OUT)
