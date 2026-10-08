import io
import json
import os
import tempfile
import unittest
from PIL import Image
from unittest.mock import patch

from lyra.artwork import textutils, scoring
from lyra.artwork.scoring import score_candidate
import lyra.artwork.providers as prov
import lyra.artwork.images as images_mod


def _itunes_hit(artist="Taylor Swift", title="Cruel Summer", tid=1):
    return {
        "trackId": tid,
        "artistName": artist,
        "trackName": title,
        "collectionName": "Lover",
        "releaseDate": "2019-08-23T07:00:00Z",
        "artworkUrl100": f"http://x/{tid}100x100bb.jpg",
        "primaryGenreName": "Pop",
    }


def _deezer_track(artist="Sơn Tùng M-TP", title="Chúng Ta Của Hiện Tại", did=7):
    return {
        "id": did,
        "title": title,
        "artist": {"name": artist},
        "album": {
            "title": "Chúng Ta Của Hiện Tại - Single",
            "cover_xl": f"http://dz/{did}/1000x1000.jpg",
            "cover_big": f"http://dz/{did}/500x500.jpg",
        },
    }


class TestTextUtils(unittest.TestCase):
    def test_suffix_strip_only_trailing(self):
        self.assertEqual(
            textutils.strip_suffix_tags("Sparkle (Official Music Video)"), "Sparkle"
        )
        # Mid-string parens are content, not tags: must survive.
        self.assertEqual(
            textutils.strip_suffix_tags("Love (Is Gone) [4K]"), "Love (Is Gone)"
        )

    def test_hyphen_preserved(self):
        self.assertEqual(textutils.ascii_fold("Sơn Tùng M-TP"), "son tung m-tp")

    def test_norm_key(self):
        self.assertEqual(
            textutils.norm_key("Chúng Ta Của Hiện Tại (Official MV)"),
            "chung ta cua hien tai",
        )


class TestScoring(unittest.TestCase):
    def test_exact_match_high(self):
        s = score_candidate("Taylor Swift", "Cruel Summer",
                            "Taylor Swift", "Cruel Summer")
        self.assertGreaterEqual(s, 90)

    def test_diacritics_match(self):
        s = score_candidate("Son Tung M-TP", "Chung Ta Cua Hien Tai",
                            "Sơn Tùng M-TP", "Chúng Ta Của Hiện Tại")
        self.assertGreaterEqual(s, 78.0)  # auto-pick band

    def test_wrong_song_rejected(self):
        self.assertEqual(
            score_candidate("Taylor Swift", "Cruel Summer",
                            "Olivia Rodrigo", "drivers license"), 0.0)

    def test_wrong_title_same_artist_rejected(self):
        self.assertEqual(
            score_candidate("Taylor Swift", "Cruel Summer",
                            "Taylor Swift", "Shake It Off"), 0.0)

    def test_version_penalty_keeps_relevant(self):
        # Taylor's Version should still pass, just score a bit lower.
        base = score_candidate("Taylor Swift", "Cruel Summer",
                               "Taylor Swift", "Cruel Summer")
        ver = score_candidate("Taylor Swift", "Cruel Summer",
                              "Taylor Swift", "Cruel Summer Taylor's Version")
        self.assertGreater(ver, 60.0)
        self.assertLess(ver, base)

    def test_empty_rejected(self):
        self.assertEqual(score_candidate("A", "", "A", "Something"), 0.0)

    def test_free_text_artist_match(self):
        s = score_candidate("", "Son Tung", "Sơn Tùng M-TP", "Lạc Trôi")
        self.assertGreaterEqual(s, 75.0)

    def test_free_text_combined_match(self):
        s = score_candidate("", "Son Tung Lac Troi", "Sơn Tùng M-TP", "Lạc Trôi")
        self.assertGreaterEqual(s, 85.0)

    def test_free_text_unrelated_rejected(self):
        s = score_candidate("", "Taylor Swift", "Sơn Tùng M-TP", "Lạc Trôi")
        self.assertEqual(s, 0.0)


class TestProviders(unittest.TestCase):
    def test_deezer_mapping(self):
        with patch.object(prov, "_deezer_raw", return_value=[_deezer_track()]):
            cands = prov._deezer_candidates("Sơn Tùng M-TP", "Chúng Ta Của Hiện Tại")
        self.assertEqual(len(cands), 1)
        self.assertEqual(cands[0]["source"], "deezer")
        self.assertTrue(cands[0]["cover_url"].endswith("1000x1000.jpg"))
        self.assertEqual(cands[0]["cover_url_fallback"], "http://dz/7/500x500.jpg")

    def test_fanout_scores_and_sorts(self):
        with patch.object(prov, "_itunes_raw", return_value=[_itunes_hit()]), \
             patch.object(prov, "_deezer_raw", return_value=[_deezer_track(
                 artist="Someone Else", title="Totally Different Song")]), \
             patch.object(prov, "_mb_release_ids", return_value=[]):
            cands = prov.search_candidates("Taylor Swift", "Cruel Summer",
                                           use_cache=False)
        self.assertTrue(cands)
        self.assertEqual(cands[0]["source"], "itunes")
        self.assertGreater(cands[0]["score"], 60)

    def test_caa_prefers_1200_thumbnail(self):
        thumbs = {"500": "http://caa/500.jpg", "1200": "http://caa/1200.jpg",
                  "small": "http://caa/small.jpg"}
        payload = {"images": [{"front": True, "thumbnails": thumbs,
                               "image": "http://caa/full.jpg"}]}
        with patch.object(prov, "_get_json", return_value=payload):
            self.assertEqual(
                prov._caa_front_url("some-mbid"), "http://caa/1200.jpg")

    def test_caa_mb_score_blending(self):
        # Same echoed query text; a weak MB match must score well below
        # an exact one instead of hiding behind the echo.
        cand = {"source": "caa", "artist": "A", "title": "B", "album": "",
                "year": "", "genre": "", "cover_url": "http://caa/x.jpg",
                "cover_url_fallback": ""}

        def run(mb_score):
            # NOTE: patch _PROVIDERS, not the module attr: search_candidates
            # dispatches through the dict holding the original refs.
            with patch.dict(prov._PROVIDERS,
                            {"caa": lambda a, t: [dict(cand, mb_score=mb_score)]}):
                return prov.search_candidates("A", "B", sources=("caa",),
                                              use_cache=False)

        hi, lo = run(98.0), run(40.0)
        self.assertTrue(hi and lo)
        self.assertGreater(hi[0]["score"], 90)
        self.assertLess(lo[0]["score"], hi[0]["score"])
        self.assertNotIn("mb_score", hi[0])  # internal signal, not output

    def test_cache_shortcircuits_network(self):
        with tempfile.TemporaryDirectory() as td:
            with patch.dict(os.environ, {"XDG_CACHE_HOME": td}):
                with patch.object(prov, "_itunes_raw", return_value=[_itunes_hit()]), \
                     patch.object(prov, "_deezer_raw", return_value=[]), \
                     patch.object(prov, "_mb_release_ids", return_value=[]):
                    first = prov.search_candidates("Taylor Swift", "Cruel Summer",
                                                   sources=("itunes",), use_cache=True)
                self.assertTrue(first)
                # Network now explodes: cached meta must still serve.
                with patch.object(prov, "_itunes_raw", side_effect=AssertionError("net!")), \
                     patch.object(prov, "_deezer_raw", side_effect=AssertionError("net!")), \
                     patch.object(prov, "_mb_release_ids", side_effect=AssertionError("net!")):
                    second = prov.search_candidates("Taylor Swift", "Cruel Summer",
                                                    sources=("itunes",), use_cache=True)
                self.assertEqual(first, second)


class FakeHeaders(dict):
    def get(self, k, default=None):
        return super().get(k, default)


class FakeResp:
    def __init__(self, data, ctype="image/jpeg"):
        self._data = data
        self.headers = FakeHeaders({"Content-Type": ctype,
                                    "Content-Length": str(len(data))})

    def __enter__(self):
        return self

    def __exit__(self, *a):
        return False

    def read(self, n=-1):
        return self._data if n is None or n < 0 else self._data[:n]


def _jpeg_bytes(w=1600, h=900, color=(200, 100, 50)):
    buf = io.BytesIO()
    Image.new("RGB", (w, h), color=color).save(buf, "JPEG")
    return buf.getvalue()


class TestImages(unittest.TestCase):
    def test_normalize_to_square_1000(self):
        with tempfile.TemporaryDirectory() as td:
            with patch.dict(os.environ, {"XDG_CACHE_HOME": td}):
                out = os.path.join(td, "cover.jpg")
                with patch("urllib.request.urlopen", return_value=FakeResp(_jpeg_bytes())):
                    self.assertTrue(images_mod.download_cover_image("http://x/a.jpg", out))
                with Image.open(out) as img:
                    self.assertEqual(img.size, (1000, 1000))

    def test_non_image_rejected(self):
        with tempfile.TemporaryDirectory() as td:
            with patch.dict(os.environ, {"XDG_CACHE_HOME": td}):
                out = os.path.join(td, "cover.jpg")
                with patch("urllib.request.urlopen",
                           return_value=FakeResp(b"<html>nope</html>", ctype="text/html")):
                    self.assertFalse(images_mod.download_cover_image("http://x/a.jpg", out))
                self.assertFalse(os.path.exists(out))

    def test_best_cover_tries_fallback_url(self):
        with tempfile.TemporaryDirectory() as td:
            with patch.dict(os.environ, {"XDG_CACHE_HOME": td}):
                out = os.path.join(td, "cover.jpg")
                cand = {
                    "source": "itunes", "artist": "A", "title": "B", "album": "Al",
                    "year": "", "genre": "", "score": 95.0,
                    "cover_url": "http://x/1000.jpg",
                    "cover_url_fallback": "http://x/600.jpg",
                }
                from lyra import artwork as aw
                with patch.object(aw, "search_candidates", return_value=[cand]):
                    calls = []

                    def fake_dl(url, path, timeout=8.0):
                        calls.append(url)
                        if url == "http://x/1000.jpg":
                            return False  # e.g. old catalog 404
                        with open(path, "wb") as f:
                            f.write(_jpeg_bytes(600, 600))
                        return True

                    with patch.object(aw, "download_cover_image", side_effect=fake_dl):
                        won = aw.download_best_cover("A", "B", out,
                                                     sources=("itunes",),
                                                     use_cache=False)
                    self.assertIsNotNone(won)
                    self.assertEqual(calls, ["http://x/1000.jpg", "http://x/600.jpg"])


class TestCoreContract(unittest.TestCase):
    def test_cover_sources_map(self):
        from lyra.core import COVER_SOURCES
        self.assertEqual(set(COVER_SOURCES["auto"]), {"itunes", "deezer", "caa"})
        self.assertEqual(COVER_SOURCES["youtube"], ())

    def test_pipeline_accepts_new_kwargs(self):
        from lyra.core import LyraPipeline
        with tempfile.TemporaryDirectory() as td:
            p = LyraPipeline(output_dir=td, cover_source="deezer",
                             cover_url="http://x/a.jpg", no_cover_search=True)
            self.assertEqual(p.cover_source, "deezer")
            self.assertEqual(p.custom_cover, "http://x/a.jpg")
            self.assertTrue(p.no_cover_search)

    def test_prepare_cover_art_local_image(self):
        from lyra.artwork import prepare_cover_art
        from PIL import Image
        with tempfile.TemporaryDirectory() as td:
            src_img = os.path.join(td, "source.png")
            out_img = os.path.join(td, "target.jpg")
            img = Image.new("RGB", (300, 200), color="blue")
            img.save(src_img, "PNG")

            ok = prepare_cover_art(src_img, out_img)
            self.assertTrue(ok)
            self.assertTrue(os.path.exists(out_img))
            with Image.open(out_img) as res:
                self.assertEqual(res.size, (1000, 1000))


if __name__ == "__main__":
    unittest.main()
