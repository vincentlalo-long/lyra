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

3
00:00:59.999 --> 00:01:02.000
Rollover line
"""
        lrc = vtt_to_lrc(sample_vtt)
        expected = "[00:01.23] Hello & world\n[00:05.10] This is a test\n[01:00.00] Rollover line"
        self.assertEqual(lrc.strip(), expected)

    def test_search_lrclib_splits_stem_and_prefers_synced(self):
        from unittest.mock import patch
        import lyra.lyric as lyr

        fake_api_hits = [
            {
                "artistName": "Sơn Tùng M-TP",
                "trackName": "Lạc Trôi",
                "albumName": "Lạc Trôi - Single",
                "duration": 233,
                "plainLyrics": "Người theo hương hoa mây mù giăng lối",
                "syncedLyrics": "",
            },
            {
                "artistName": "Sơn Tùng M-TP",
                "trackName": "Lạc Trôi",
                "albumName": "Lạc Trôi - Single",
                "duration": 233,
                "plainLyrics": "Người theo hương hoa mây mù giăng lối",
                "syncedLyrics": "[00:15.20] Người theo hương hoa mây mù giăng lối",
            },
        ]
        with patch.object(lyr, "_lrclib_request", return_value=fake_api_hits):
            # Even when caller passes identical stem for artist and title, it splits and ranks synced first
            results = lyr.search_lrclib(
                "Sơn Tùng M-TP - Lạc Trôi (Official MV)",
                "Sơn Tùng M-TP - Lạc Trôi (Official MV)",
            )
        self.assertEqual(len(results), 2)
        self.assertTrue(results[0]["synced"])
        self.assertIn("[00:15.20]", results[0]["lyrics"])
        self.assertFalse(results[1]["synced"])

    def test_segments_to_lrc_strips_hallucinations_and_sorts(self):
        from lyra.lyric import _segments_to_lrc

        segs = [
            {"start": 45.0, "end": 48.0, "text": "Second line", "no_speech_prob": 0.1, "avg_logprob": -0.2},
            {"start": 10.0, "end": 14.0, "text": "First line Hãy subscribe cho kênh Ghiền Mì Gõ Để không bỏ lỡ những video hấp dẫn", "no_speech_prob": 0.1, "avg_logprob": -0.2},
            {"start": 18.0, "end": 22.0, "text": "H subscribe cho k Ghi M G kh b l nh video h d", "no_speech_prob": 0.1, "avg_logprob": -0.2},
            {"start": 24.0, "end": 27.0, "text": "tóc, tóc, tóc, tóc, tóc, tóc", "no_speech_prob": 0.1, "avg_logprob": -0.2},
            {"start": 20.0, "end": 23.0, "text": "Tập 3. Tập 3", "no_speech_prob": 0.1, "avg_logprob": -0.2},
            {"start": 25.0, "end": 28.0, "text": "Thank you. Thank you.", "no_speech_prob": 0.1, "avg_logprob": -0.2},
        ]
        lrc = _segments_to_lrc(segs)
        lines = lrc.splitlines()
        self.assertEqual(lines[0], "[00:10.00] First line")
        self.assertEqual(lines[-1], "[00:45.00] Second line")
        self.assertNotIn("Ghiền Mì Gõ", lrc)
        self.assertNotIn("Ghi M G", lrc)
        self.assertNotIn("tóc", lrc)
        self.assertNotIn("Tập 3", lrc)
        self.assertNotIn("Thank you", lrc)

    def test_verify_lrc_rejects_corrupt_or_reversed_without_network(self):
        from lyra.lyric import verify_lrc_with_audio

        bad_halluc = "[00:10.00] First line\n[00:20.00] hãy subscribe cho kênh lalaschool Để không bỏ lỡ\n"
        ok, conf, reason = verify_lrc_with_audio("nonexistent.mp3", bad_halluc, api_key="test")
        self.assertFalse(ok)
        self.assertEqual(conf, 0.0)
        self.assertIn("hallucinated", reason.lower())

        bad_reversed = (
            "[00:10.00] Line 1\n"
            "[00:20.00] Line 2\n"
            "[02:45.00] Line 3\n"
            "[02:30.00] Line 4\n"
        )
        ok2, conf2, reason2 = verify_lrc_with_audio("nonexistent.mp3", bad_reversed, api_key="test")
        self.assertFalse(ok2)
        self.assertEqual(conf2, 0.0)
        self.assertIn("timestamps", reason2.lower())

    def test_verify_lrc_plain_and_timed_with_mock_audio(self):
        import io
        import json
        import tempfile
        from unittest.mock import patch
        from lyra.lyric import verify_lrc_with_audio

        class _FakeResp:
            def __init__(self, text):
                self._b = json.dumps({"text": text}).encode("utf-8")
            def __enter__(self): return self
            def __exit__(self, *a): return False
            def read(self): return self._b

        with tempfile.TemporaryDirectory() as td:
            mp3 = os.path.join(td, "song.mp3")
            with open(mp3, "wb") as f:
                f.write(b"\xff\xfb\x90\x00" + b"\x00" * 2048)

            def _fake_ffmpeg(cmd, **kwargs):
                out_path = cmd[-1]
                with open(out_path, "wb") as f:
                    f.write(b"\xff\xfb\x90\x00" + b"\x00" * 1024)

            plain_lrc = "Who we are tonight\nI don't wanna know where you go\nWithout each other it is just too hard\nLeave here without you forever"
            with patch("shutil.which", return_value="/usr/bin/ffmpeg"), \
                 patch("subprocess.run", side_effect=_fake_ffmpeg), \
                 patch("urllib.request.urlopen", return_value=_FakeResp("completely different vietnamese song lyrics")):
                ok, conf, _ = verify_lrc_with_audio(mp3, plain_lrc, api_key="gsk_test")
            self.assertFalse(ok)
            self.assertLess(conf, 0.28)

            timed_lrc = "[00:15.00] Every time we kiss you make me a believer\n[00:20.00] Let the music hear me take my body deeper\n[00:25.00] Run me like a fever fever fever\n"
            with patch("shutil.which", return_value="/usr/bin/ffmpeg"), \
                 patch("subprocess.run", side_effect=_fake_ffmpeg), \
                 patch("urllib.request.urlopen", return_value=_FakeResp("Every time we kiss you make me a believer Let the music hear me")):
                ok2, conf2, _ = verify_lrc_with_audio(mp3, timed_lrc, api_key="gsk_test")
            self.assertTrue(ok2)
            self.assertGreaterEqual(conf2, 0.50)

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

    def test_genre_sidecar_roundtrip(self):
        from lyra import genres
        with tempfile.TemporaryDirectory() as tmp:
            sidecar = os.path.join(tmp, "genres.json")
            self.assertEqual(genres.get_genres("/m/a.mp3", path=sidecar), [])
            genres.set_genres("/m/a.mp3", ["lo-fi", " rain ", ""], path=sidecar)
            self.assertEqual(genres.get_genres("/m/a.mp3", path=sidecar), ["lo-fi", "rain"])
            # Empty clears the entry.
            genres.set_genres("/m/a.mp3", [], path=sidecar)
            self.assertEqual(genres.get_genres("/m/a.mp3", path=sidecar), [])
            # Corrupt file reads as empty, never raises.
            with open(sidecar, "w") as f:
                f.write("{broken")
            self.assertEqual(genres.load_sidecar(path=sidecar), {})

    def test_genre_backfill_uses_itunes_and_skips_tagged(self):
        from unittest.mock import patch
        from lyra import genres
        with tempfile.TemporaryDirectory() as tmp:
            sidecar = os.path.join(tmp, "genres.json")
            tagged = os.path.join(tmp, "Known - Song.mp3")
            fresh = os.path.join(tmp, "Taylor Swift - Cruel Summer.mp3")
            open(tagged, "w").close()
            open(fresh, "w").close()
            genres.set_genres(tagged, ["rock"], path=sidecar)
            fake_info = {"genre": "Pop"}
            with patch("lyra.image.search_itunes_cover", return_value=fake_info):
                stats = genres.backfill_directory(tmp, path=sidecar)
            self.assertEqual(stats["scanned"], 2)
            self.assertEqual(stats["skipped"], 1)
            self.assertEqual(stats["filled"], 1)
            self.assertEqual(genres.get_genres(fresh, path=sidecar), ["Pop"])
            # Pre-tagged entry untouched.
            self.assertEqual(genres.get_genres(tagged, path=sidecar), ["rock"])

    def test_tagger_writes_genre_tcon(self):
        from lyra.tagger import tag_audio_file
        from mutagen.id3 import ID3
        with tempfile.TemporaryDirectory() as tmp:
            mp3 = os.path.join(tmp, "t.mp3")
            # Minimal valid MP3 frame so mutagen accepts the file.
            with open(mp3, "wb") as f:
                f.write(bytes.fromhex("fffb9000000000000000000000000000000000000000000000000000000000000000"))
            self.assertTrue(tag_audio_file(mp3, "T", "A", genre="Lo-Fi"))
            self.assertEqual(ID3(mp3)["TCON"].text, ["Lo-Fi"])
            # No genre -> no TCON frame (fresh file).
            mp3b = os.path.join(tmp, "t2.mp3")
            with open(mp3b, "wb") as f:
                f.write(bytes.fromhex("fffb9000000000000000000000000000000000000000000000000000000000000000"))
            self.assertTrue(tag_audio_file(mp3b, "T", "A"))
            self.assertNotIn("TCON", ID3(mp3b))


if __name__ == "__main__":
    unittest.main()
