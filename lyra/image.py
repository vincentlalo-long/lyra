
from PIL import Image, ImageFilter, ImageEnhance
from typing import Optional


def process_cover_art(
    input_path: str,
    output_path: str,
    mode: str = "blur_pad",
    target_size: int = 1000
) -> bool:
    try:
        with Image.open(input_path) as img:
            img = img.convert("RGB")
            w, h = img.size

            if w == h and w >= target_size:
                # Already square
                img.save(output_path, "JPEG", quality=95)
                return True

            if mode == "center_crop":
                min_dim = min(w, h)
                left = (w - min_dim) // 2
                top = (h - min_dim) // 2
                cropped = img.crop((left, top, left + min_dim, top + min_dim))
                resized = cropped.resize((target_size, target_size), Image.Resampling.LANCZOS)
                resized.save(output_path, "JPEG", quality=95)
                return True

            # Default: blur_pad
            # 1. Background: Cover the square canvas, blur and dim slightly
            bg = img.resize((target_size, target_size), Image.Resampling.BILINEAR)
            bg = bg.filter(ImageFilter.GaussianBlur(radius=35))
            enhancer = ImageEnhance.Brightness(bg)
            bg = enhancer.enhance(0.65)  # Dim background by 35% for contrast

            # 2. Foreground: Fit original image into the square canvas
            ratio = min(target_size / w, target_size / h)
            new_w = int(w * ratio)
            new_h = int(h * ratio)
            fg = img.resize((new_w, new_h), Image.Resampling.LANCZOS)

            # 3. Paste foreground into center of background
            pos_x = (target_size - new_w) // 2
            pos_y = (target_size - new_h) // 2
            bg.paste(fg, (pos_x, pos_y))

            bg.save(output_path, "JPEG", quality=95)
            return True
    except Exception as e:
        import sys
        print(f"warning: failed to process cover art: {e}", file=sys.stderr)
        return False
