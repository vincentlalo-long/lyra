import unittest
from unittest.mock import patch

from lyra import lrclib, search


class TestSearch(unittest.TestCase):
    def test_deezer_mapping(self):
        payload = {
            "data": [
                {
                    "id": 123,
                    "title": "Cruel Summer",
                    "artist": {"name": "Taylor Swift"},
                    "album": {
                        "title": "Lover",
                        "cover_xl": "https://cdn/cover.jpg",
                    },
                    "duration": 178,
                    "link": "https://deezer/track/123",
                    "isrc": "USUG11901491",
                }
            ]
        }
        with patch.object(search, "_get_json", return_value=payload):
            items = search._search_deezer("Taylor Swift Cruel Summer", 5)
        self.assertEqual(len(items), 1)
        item = items[0]
        self.assertEqual(item["source"], "deezer")
        self.assertEqual(item["artist"], "Taylor Swift")
        self.assertEqual(item["title"], "Cruel Summer")
        self.assertEqual(item["album"], "Lover")
        self.assertEqual(item["duration"], "02:58")
        self.assertEqual(item["duration_secs"], 178)
        self.assertEqual(item["cover_url"], "https://cdn/cover.jpg")
        # Official results need a YouTube resolve before download.
        self.assertEqual(item["url"], "https://deezer/track/123")
        self.assertIsInstance(item["id"], str)

    def test_itunes_mapping_and_cover_upgrade(self):
        payload = {
            "results": [
                {
                    "trackId": 42,
                    "artistName": "Taylor Swift",
                    "trackName": "Cruel Summer",
                    "collectionName": "Lover",
                    "releaseDate": "2019-08-23T07:00:00Z",
                    "artworkUrl100": "https://art/100x100bb.jpg",
                    "trackTimeMillis": 178000,
                    "trackViewUrl": "https://music.apple.com/song/42",
                }
            ]
        }
        with patch.object(search, "_get_json", return_value=payload):
            items = search._search_itunes("cruel summer", 5)
        self.assertEqual(items[0]["source"], "itunes")
        self.assertEqual(items[0]["year"], "2019")
        self.assertEqual(items[0]["cover_url"], "https://art/1000x1000bb.jpg")
        self.assertEqual(items[0]["duration_secs"], 178)

    def test_musicbrainz_mapping_and_video_filter(self):
        payload = {
            "recordings": [
                {
                    "id": "mb1",
                    "title": "Cruel Summer",
                    "length": 178000,
                    "artist-credit": [
                        {"name": "Taylor Swift", "joinphrase": ""}
                    ],
                    "releases": [
                        {
                            "id": "rel1",
                            "title": "Lover",
                            "date": "2019-08-23",
                            "status": "Official",
                        }
                    ],
                    "isrcs": ["USUG11901491"],
                },
                {
                    "id": "mbvideo",
                    "title": "Cruel Summer (video)",
                    "length": 90000,
                    "video": True,
                    "artist-credit": [{"name": "Taylor Swift"}],
                    "releases": [],
                },
            ]
        }
        with patch.object(search, "_musicbrainz_get", return_value=payload):
            items = search._search_musicbrainz("Taylor Swift Cruel Summer", 5)
        self.assertEqual(len(items), 1)
        item = items[0]
        self.assertEqual(item["source"], "musicbrainz")
        self.assertEqual(item["artist"], "Taylor Swift")
        self.assertEqual(item["album"], "Lover")
        self.assertEqual(item["year"], "2019")
        self.assertEqual(item["duration"], "02:58")
        self.assertEqual(item["isrc"], "USUG11901491")
        self.assertEqual(item["url"], "https://musicbrainz.org/recording/mb1")
        self.assertIsInstance(item["id"], str)

    def test_spotify_mapping(self):
        payload = {
            "tracks": {
                "items": [
                    {
                        "id": "sp1",
                        "name": "Cruel Summer",
                        "duration_ms": 178000,
                        "external_urls": {"spotify": "https://open.spotify.com/track/sp1"},
                        "external_ids": {"isrc": "USUG11901491"},
                        "artists": [{"name": "Taylor Swift"}],
                        "album": {
                            "name": "Lover",
                            "release_date": "2019-08-23",
                            "images": [
                                {"url": "https://img/small.jpg", "width": 64},
                                {"url": "https://img/large.jpg", "width": 640},
                            ],
                        },
                    }
                ]
            }
        }
        with patch.object(search, "_spotify_token", return_value="token"), \
                patch.object(search, "_get_json", return_value=payload):
            items = search._search_spotify("cruel summer", 5)
        self.assertEqual(items[0]["source"], "spotify")
        self.assertEqual(items[0]["album"], "Lover")
        self.assertEqual(items[0]["cover_url"], "https://img/large.jpg")
        self.assertEqual(items[0]["duration"], "02:58")

    def test_auto_prefers_official_and_skips_youtube(self):
        official = [
            {
                "id": "deezer:1",
                "url": "",
                "title": "Cruel Summer",
                "artist": "Taylor Swift",
                "uploader": "Lover",
                "duration": "02:58",
                "duration_secs": 178,
                "source": "deezer",
                "album": "Lover",
                "year": "",
                "cover_url": "",
            }
        ]
        with patch.object(search, "spotify_available", return_value=False), \
                patch.dict(search._PROVIDERS, {
                    "musicbrainz": lambda q, l: [],
                    "deezer": lambda q, l: official,
                    "itunes": lambda q, l: [],
                }), \
                patch.object(search, "_search_youtube") as yt:
            items = search.search_music("Taylor Swift Cruel Summer", 5, "auto")
        self.assertEqual(len(items), 1)
        self.assertEqual(items[0]["source"], "deezer")
        yt.assert_not_called()

    def test_auto_falls_back_to_youtube(self):
        yt_items = [
            {
                "id": "yt1",
                "url": "https://youtube/watch?v=yt1",
                "title": "Cruel Summer",
                "artist": "Taylor Swift",
                "uploader": "Taylor Swift",
                "duration": "02:58",
                "duration_secs": 178,
                "source": "youtube",
            }
        ]
        with patch.object(search, "spotify_available", return_value=False), \
                patch.dict(search._PROVIDERS, {
                    "musicbrainz": lambda q, l: [],
                    "deezer": lambda q, l: [],
                    "itunes": lambda q, l: [],
                }), \
                patch.object(search, "_search_youtube", return_value=yt_items):
            items = search.search_music("Taylor Swift Cruel Summer", 5, "auto")
        self.assertEqual(items[0]["source"], "youtube")
        self.assertEqual(items[0]["url"], "https://youtube/watch?v=yt1")

    def test_explicit_youtube_skips_official(self):
        with patch.object(search, "_search_youtube", return_value=[]) as yt, \
                patch.object(search, "_search_deezer") as dz:
            search.search_music("anything", 5, "youtube")
        yt.assert_called_once()
        dz.assert_not_called()

    def test_spotify_without_credentials_raises_helpful_error(self):
        with patch.object(search, "spotify_available", return_value=False):
            with self.assertRaises(RuntimeError) as ctx:
                search.search_music("anything", 5, "spotify")
        self.assertIn("SPOTIFY_CLIENT_ID", str(ctx.exception))

    def test_resolve_uses_youtube_match_and_lrclib(self):
        yt = {
            "id": "yt1",
            "url": "https://youtube/watch?v=yt1",
            "title": "Cruel Summer",
            "artist": "Taylor Swift",
            "uploader": "Taylor Swift",
            "raw_title": "Taylor Swift - Cruel Summer",
            "duration": "02:58",
            "duration_secs": 178,
        }
        rec = {"album": "Lover", "has_synced": True, "synced_lyrics": "[00:01.00] hi"}
        with patch("lyra.core.search_youtube", return_value=[yt]), \
                patch.object(lrclib, "best_match", return_value=rec):
            info = search.resolve("Taylor Swift", "Cruel Summer", duration=178)
        self.assertEqual(info["url"], "https://youtube/watch?v=yt1")
        self.assertEqual(info["album"], "Lover")
        self.assertTrue(info["has_lyrics"])


class TestLrclib(unittest.TestCase):
    def test_exact_record_normalization(self):
        raw = {
            "id": 1,
            "trackName": "Cruel Summer",
            "artistName": "Taylor Swift",
            "albumName": "Lover",
            "duration": 178,
            "instrumental": False,
            "plainLyrics": "hi",
            "syncedLyrics": "[00:01.00] hi",
        }
        with patch.object(lrclib, "_get_json", return_value=raw):
            rec = lrclib.get_exact("Taylor Swift", "Cruel Summer", "Lover", 178)
        self.assertEqual(rec["title"], "Cruel Summer")
        self.assertEqual(rec["album"], "Lover")
        self.assertTrue(rec["has_synced"])
        self.assertEqual(lrclib.to_lrc(rec), "[00:01.00] hi")

    def test_best_match_falls_back_to_search_and_ranks_duration(self):
        near = {
            "id": 2,
            "trackName": "Cruel Summer",
            "artistName": "Taylor Swift",
            "albumName": "Lover",
            "duration": 179,
            "syncedLyrics": "[00:01.00] hi",
            "plainLyrics": "",
            "instrumental": False,
        }
        far = {
            "id": 3,
            "trackName": "Cruel Summer (Live)",
            "artistName": "Taylor Swift",
            "albumName": "Live",
            "duration": 260,
            "syncedLyrics": "",
            "plainLyrics": "",
            "instrumental": False,
        }
        with patch.object(lrclib, "get_exact", return_value=None), \
                patch.object(lrclib, "search", return_value=[
                    lrclib._record(far), lrclib._record(near)
                ]):
            rec = lrclib.best_match("Taylor Swift", "Cruel Summer", duration=178)
        self.assertEqual(rec["id"], 2)

    def test_probe_reports_synced_availability(self):
        rec = {"has_synced": True, "has_plain": True, "album": "Lover", "duration": 178}
        with patch.object(lrclib, "best_match", return_value=rec):
            probe = lrclib.probe("Taylor Swift", "Cruel Summer")
        self.assertTrue(probe["has_manual"])
        self.assertTrue(probe["has_auto"])
        self.assertEqual(probe["langs"], [])


if __name__ == "__main__":
    unittest.main()
