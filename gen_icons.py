"""生成 Tauri 所需的应用图标（占位图标，之后可用 `npm run tauri icon 自己的图.png` 替换）。

产物：
- Windows/通用：32x32.png / 128x128.png / 128x128@2x.png / icon.png / icon.ico
- macOS：icon.icns（.app/.dmg 打包必需）、tray-mac.png（菜单栏 template 图）
"""
import os
import struct
from io import BytesIO

from PIL import Image, ImageDraw, ImageFont

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "src-tauri", "icons")

CJK_FONTS = [
    "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Bold.ttc",
    "/System/Library/Fonts/PingFang.ttc",
    "/System/Library/Fonts/STHeiti Medium.ttc",
    "/mnt/c/Windows/Fonts/msyh.ttc",
    "/mnt/c/Windows/Fonts/simhei.ttf",
    "C:/Windows/Fonts/msyh.ttc",
    "C:/Windows/Fonts/simhei.ttf",
]


def load_font(size):
    """返回 (font, text)；没有中文字体时降级成 'A'。"""
    for path in CJK_FONTS:
        try:
            return ImageFont.truetype(path, size), "译"
        except Exception:
            continue
    return ImageFont.load_default(), "A"


def draw_centered(img, txt, font, fill):
    d = ImageDraw.Draw(img)
    bbox = d.textbbox((0, 0), txt, font=font)
    tw, th = bbox[2] - bbox[0], bbox[3] - bbox[1]
    d.text(
        ((img.width - tw) / 2 - bbox[0], (img.height - th) / 2 - bbox[1]),
        txt,
        font=font,
        fill=fill,
    )


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
    font, txt = load_font(int(size * 0.6))
    draw_centered(img, txt, font, (255, 255, 255, 255))
    return img


def make_tray_template(size=36):
    """macOS 菜单栏 template 图：纯黑字形 + 透明背景。

    系统只读取 alpha 通道，按菜单栏明暗自动反色，所以这里不能用彩色图。
    36px = 18pt @2x，留一点内边距免得贴边。
    """
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    font, txt = load_font(int(size * 0.86))
    draw_centered(img, txt, font, (0, 0, 0, 255))
    return img


# ICNS 里各段的类型码 -> 像素尺寸（都用 PNG 负载，macOS 10.7+ 支持）
ICNS_TYPES = [
    (b"ic11", 32),    # 16pt @2x
    (b"ic12", 64),    # 32pt @2x
    (b"ic07", 128),   # 128pt @1x
    (b"ic13", 256),   # 128pt @2x
    (b"ic08", 256),   # 256pt @1x
    (b"ic14", 512),   # 256pt @2x
    (b"ic09", 512),   # 512pt @1x
    (b"ic10", 1024),  # 512pt @2x
]


def save_icns(base, path):
    """手写 ICNS 容器（头 + 若干 PNG 段）。

    不依赖 macOS 的 iconutil，也不依赖 Pillow 的 ICNS 保存支持 —— 这样在
    Windows/WSL 上也能生成，Mac 那边直接拿来打包。
    """
    chunks = []
    for code, px in ICNS_TYPES:
        buf = BytesIO()
        base.resize((px, px), Image.LANCZOS).save(buf, format="PNG")
        data = buf.getvalue()
        chunks.append(code + struct.pack(">I", len(data) + 8) + data)
    body = b"".join(chunks)
    with open(path, "wb") as f:
        f.write(b"icns" + struct.pack(">I", len(body) + 8) + body)


os.makedirs(OUT, exist_ok=True)
base = make_base(1024)
base.resize((32, 32), Image.LANCZOS).save(os.path.join(OUT, "32x32.png"))
base.resize((128, 128), Image.LANCZOS).save(os.path.join(OUT, "128x128.png"))
base.resize((256, 256), Image.LANCZOS).save(os.path.join(OUT, "128x128@2x.png"))
base.resize((512, 512), Image.LANCZOS).save(os.path.join(OUT, "icon.png"))
# Windows .ico（多尺寸）
base.resize((256, 256), Image.LANCZOS).save(
    os.path.join(OUT, "icon.ico"),
    sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
)
# macOS
save_icns(base, os.path.join(OUT, "icon.icns"))
make_tray_template().save(os.path.join(OUT, "tray-mac.png"))
print("icons generated in", OUT)
