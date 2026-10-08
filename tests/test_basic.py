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


    def test_clean_search_term(self):
        from lyra.image import clean_search_term
        cleaned = clean_search_term("Sơn Tùng M-TP - Chúng Ta Của Hiện Tại (Official Music Video)")
        # Hyphen inside names is preserved, " - " separator and suffix tag go.
        self.assertEqual(cleaned, "Sơn Tùng M-TP Chúng Ta Của Hiện Tại")

    def test_search_itunes_cover(self):
        # Deterministic: realistic iTunes payload, no network.
        from unittest.mock import patch
        import lyra.artwork.providers as prov
        payload = [{
            "trackId": 1468058165,
            "artistName": "Taylor Swift",
            "trackName": "Cruel Summer",
            "collectionName": "Lover",
            "releaseDate": "2019-08-23T07:00:00Z",
            "artworkUrl100": "https://is1-ssl.mzstatic.com/image/thumb/100x100bb.jpg",
            "primaryGenreName": "Pop",
        }]
        with tempfile.TemporaryDirectory() as td:
            with patch.dict(os.environ, {"XDG_CACHE_HOME": td}):
                with patch.object(prov, "_itunes_raw", return_value=payload):
                    from lyra.image import search_itunes_cover
                    res = search_itunes_cover("Taylor Swift", "Cruel Summer")
        self.assertIsNotNone(res)
        self.assertEqual(res["artist"], "Taylor Swift")
        self.assertEqual(res["title"], "Cruel Summer")
        self.assertTrue(res["cover_url"].endswith("1000x1000bb.jpg"))

    def test_is_relevant_result(self):
        from lyra.image import is_relevant_result
        # Exact + MV-suffixed hits pass
        self.assertTrue(is_relevant_result("Sơn Tùng M-TP", "Chúng Ta Của Hiện Tại",
                                           "Sơn Tùng M-TP", "Chúng Ta Của Hiện Tại"))
        self.assertTrue(is_relevant_result("Taylor Swift", "Cruel Summer",
                                           "Taylor Swift", "Cruel Summer (Official Video)"))
        # Wrong song / wrong artist rejected
        self.assertFalse(is_relevant_result("Taylor Swift", "Cruel Summer",
                                            "Olivia Rodrigo", "drivers license"))
        self.assertFalse(is_relevant_result("Taylor Swift", "Cruel Summer",
                                            "Taylor Swift", "Shake It Off"))
        # Empty titles never relevant
        self.assertFalse(is_relevant_result("A", "", "A", "Something"))

    def test_search_itunes_cover_skips_irrelevant_and_none_ids(self):
        from unittest.mock import patch
        import lyra.artwork.providers as prov

        wrong_hit = {"trackId": 1, "artistName": "Olivia Rodrigo", "trackName": "drivers license",
                     "collectionName": "SOUR", "releaseDate": "2021-01-01",
                     "artworkUrl100": "http://x/100x100bb.jpg", "primaryGenreName": "Pop"}
        no_id_hit = {"artistName": "Taylor Swift", "trackName": "Cruel Summer",
                     "collectionName": "Lover", "releaseDate": "2019-08-23",
                     "artworkUrl100": "http://x/cover100x100bb.jpg", "primaryGenreName": "Pop"}
        no_id_no_art = {"artistName": "Taylor Swift", "trackName": "Cruel Summer",
                        "collectionName": "Lover", "releaseDate": "2019-08-23",
                        "artworkUrl100": "", "primaryGenreName": "Pop"}

        # First hit is irrelevant (must be skipped), then two None-trackId rows:
        # entries without trackId must not shadow each other.
        with patch.object(prov, "_itunes_raw", return_value=[wrong_hit, no_id_no_art, no_id_hit]):
            res = prov.search_candidates("Taylor Swift", "Cruel Summer",
                                         sources=("itunes",), use_cache=False)
        self.assertTrue(res)
        self.assertEqual(res[0]["title"], "Cruel Summer")
        self.assertTrue(res[0]["cover_url"].endswith("1000x1000bb.jpg"))

        # Only irrelevant hits -> empty (falls back to YouTube thumbnail upstream)
        with patch.object(prov, "_itunes_raw", return_value=[wrong_hit]):
            self.assertEqual(prov.search_candidates("Taylor Swift", "Cruel Summer",
                                                    sources=("itunes",),
                                                    use_cache=False), [])


if __name__ == "__main__":
    unittest.main()
