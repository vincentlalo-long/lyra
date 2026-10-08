import unittest
from PIL import Image
import tempfile
import os

from lyra.utils import clean_title_and_artist, sanitize_filename
from lyra.lyric import vtt_to_lrc
from lyra.image import process_cover_art


class TestLyraBasic(unittest.TestCase):
    def test_clean_title(self):
        artist, title = clean_title_and_artist(
            "YOASOBI - 夜に駆ける (Official Music Video)",
            uploader="Ayase / YOASOBI"
        )
        self.assertEqual(artist, "YOASOBI")
        self.assertEqual(title, "夜に駆ける")

    def test_clean_title_fallback_uploader(self):
        artist, title = clean_title_and_artist(
            "Sparkle [4K 60FPS]",
            uploader="RADWIMPS - Topic"
        )
        self.assertEqual(artist, "RADWIMPS")
        self.assertEqual(title, "Sparkle")

    def test_vtt_to_lrc(self):
        sample_vtt = """WEBVTT
Kind: captions
Language: en

1
00:00:01.234 --> 00:00:04.567
Hello &amp; world

2
00:00:05.100 --> 00:00:08.200
<c.colorE5E5E5>This is a test</c>
"""
        lrc = vtt_to_lrc(sample_vtt)
        expected = "[00:01.23] Hello & world\n[00:05.10] This is a test"
        self.assertEqual(lrc.strip(), expected)

    def test_fetch_all_vtt_options(self):
        from lyra.lyric import fetch_all_vtt
        import lyra.lyric

        info = {
            "subtitles": {
                "vi": [{"ext": "vtt", "url": "mock://vi"}],
                "en": [{"ext": "vtt", "url": "mock://en"}],
                "ja": [{"ext": "vtt", "url": "mock://ja"}],
            },
            "automatic_captions": {
                "en": [{"ext": "vtt", "url": "mock://auto_en"}],
            }
        }

        orig_fetch = lyra.lyric._fetch_vtt_text
        try:
            lyra.lyric._fetch_vtt_text = lambda url, depth=0: f"WEBVTT\n\n00:00:01.000 --> 00:00:04.000\nSub {url}"

            # Download all manual subtitles (ordered: target langs first, then others)
            all_subs = fetch_all_vtt(info, target_langs=["vi", "en"], download_all=True)
            self.assertEqual(list(all_subs.keys()), ["vi", "en", "ja"])

            # Test allow_auto=False
            no_manual = {"subtitles": {}, "automatic_captions": {"en": [{"ext": "vtt", "url": "mock://auto_en"}]}}
            res_no_auto = fetch_all_vtt(no_manual, allow_auto=False)
            self.assertEqual(res_no_auto, {})

            # Test allow_auto=True
            res_auto = fetch_all_vtt(no_manual, allow_auto=True)
            self.assertEqual(list(res_auto.keys()), ["en"])
        finally:
            lyra.lyric._fetch_vtt_text = orig_fetch

    def test_image_cover_blur_pad(self):
        with tempfile.TemporaryDirectory() as td:
            # Create dummy 16:9 image (160x90)
            in_img_path = os.path.join(td, "test_16_9.jpg")
            out_img_path = os.path.join(td, "out_1x1.jpg")

            img = Image.new("RGB", (160, 90), color=(200, 100, 50))
            img.save(in_img_path)

            ok = process_cover_art(in_img_path, out_img_path, mode="blur_pad", target_size=500)
            self.assertTrue(ok)
            self.assertTrue(os.path.exists(out_img_path))

            with Image.open(out_img_path) as res:
                self.assertEqual(res.size, (500, 500))


if __name__ == "__main__":
    unittest.main()
