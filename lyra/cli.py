import os
import re
import sys
import json
import argparse
import urllib.request
import urllib.parse
from .core import LyraPipeline, search_youtube, get_video_info, probe_lyrics


def main():
    parser = argparse.ArgumentParser(
        prog="lyra",
        description="Download audio, generate square cover art, and extract synced lyrics.",
    )
    subparsers = parser.add_subparsers(dest="command")

    # Command: get
    get_parser = subparsers.add_parser("get", help="Fetch and process track from URL")
    get_parser.add_argument("url", type=str, help="Video URL")
    get_parser.add_argument(
        "-o", "--output",
        type=str,
        default="./output",
        help="Output directory (default: ./output)",
    )
    get_parser.add_argument(
        "--cover-mode",
        choices=["itunes", "auto", "blur_pad", "center_crop"],
        default="itunes",
        help="Cover art mode: itunes/auto (studio search, fallback YouTube), blur_pad/center_crop (YouTube only)",
    )
    get_parser.add_argument(
        "--cover-source",
        choices=["auto", "all", "itunes", "deezer", "caa", "web", "youtube"],
        default="auto",
        help="Studio providers to query (default: auto = itunes+deezer+caa; all = includes web)",
    )
    get_parser.add_argument(
        "--cover",
        type=str,
        default=None,
        help="Use this custom cover image file path or URL",
    )
    get_parser.add_argument(
        "--crop-focus",
        type=str,
        default=None,
        help="Crop focal point for center_crop as 'x y' in 0..1 (e.g. '0.3 0.7')",
    )
    get_parser.add_argument(
        "--cover-url",
        type=str,
        default=None,
        help="Use this exact cover image URL (alias for --cover)",
    )
    get_parser.add_argument(
        "--no-cover-search",
        action="store_true",
        help="Skip online cover search, use YouTube thumbnail only",
    )
    get_parser.add_argument(
        "-n", "--name",
        type=str,
        default=None,
        help="Name of the file (default: <Artist> - <Title>)",
    )
    get_parser.add_argument(
        "-t", "--title",
        type=str,
        default=None,
        help="Song title for ID3 tag (overrides parsed YouTube title)",
    )
    get_parser.add_argument(
        "-a", "--album",
        type=str,
        default=None,
        help="Album of the song (default: <Title> - Single)",
    )
    get_parser.add_argument(
        "-s", "--singer",
        type=str,
        default=None,
        help="Author / Artist of the song",
    )
    get_parser.add_argument(
        "-g", "--genre",
        type=str,
        default=None,
        help="Genre of the song",
    )
    get_parser.add_argument(
        "--no-lyrics",
        action="store_true",
        help="Skip downloading and embedding synced lyrics",
    )
    get_parser.add_argument(
        "--no-auto-lyrics",
        action="store_true",
        help="Skip YouTube auto-generated captions/lyrics (only download manual creator subtitles)",
    )
    get_parser.add_argument(
        "--lyrics-lang",
        type=str,
        default=None,
        help="Download lyrics for this language code only (e.g. 'vi', 'en', 'ja'). Overrides priority order.",
    )
    get_parser.add_argument(
        "--json",
        action="store_true",
        help="Output line-by-line JSON progress events for GUI/TUI integration",
    )

    # Command: search
    search_parser = subparsers.add_parser("search", help="Search official music sources (YouTube fallback)")
    search_parser.add_argument("query", type=str, help="Search query")
    search_parser.add_argument(
        "-l", "--limit",
        type=int,
        default=15,
        help="Number of results (default: 15)",
    )
    search_parser.add_argument(
        "-s", "--source",
        choices=["auto", "spotify", "musicbrainz", "deezer", "itunes", "youtube", "all"],
        default="auto",
        help="Source policy: auto (Spotify/MusicBrainz/Deezer/iTunes, then "
             "YouTube), an official provider, youtube, or all",
    )
    search_parser.add_argument(
        "--json",
        action="store_true",
        help="Output results as JSON",
    )

    # Command: resolve (official metadata -> downloadable YouTube match)
    resolve_parser = subparsers.add_parser(
        "resolve", help="Resolve an official music result to a downloadable video"
    )
    resolve_parser.add_argument("--artist", type=str, required=True, help="Artist name")
    resolve_parser.add_argument("--title", type=str, required=True, help="Track title")
    resolve_parser.add_argument("--album", type=str, default="", help="Album name (optional)")
    resolve_parser.add_argument("--duration", type=int, default=0, help="Duration in seconds (optional)")
    resolve_parser.add_argument("--source", type=str, default="", help="Original search source (metadata only)")
    resolve_parser.add_argument("--cover-url", type=str, default="", help="Cover URL from the search result")
    resolve_parser.add_argument("--year", type=str, default="", help="Release year from the search result")
    resolve_parser.add_argument("--json", action="store_true", help="Output info as JSON")

    # Command: info
    info_parser = subparsers.add_parser("info", help="Get track info from URL")
    info_parser.add_argument("url", type=str, help="Video URL")
    info_parser.add_argument(
        "--json",
        action="store_true",
        help="Output info as JSON",
    )

    # Command: probe-lyrics
    probe_parser = subparsers.add_parser(
        "probe-lyrics", help="Check lyric availability (LRCLIB metadata or video subtitles)"
    )
    probe_parser.add_argument("url", type=str, nargs="?", default="", help="Video URL (optional)")
    probe_parser.add_argument("--artist", type=str, default="", help="Artist name for an LRCLIB lookup")
    probe_parser.add_argument("--title", type=str, default="", help="Track title for an LRCLIB lookup")
    probe_parser.add_argument("--album", type=str, default="", help="Album name (optional)")
    probe_parser.add_argument("--duration", type=int, default=0, help="Duration in seconds (optional)")
    probe_parser.add_argument(
        "--json",
        action="store_true",
        help="Output probe result as JSON",
    )

    # Command: stage-image
    stage_parser = subparsers.add_parser("stage-image", help="Download a remote image into the local cache")
    stage_parser.add_argument("url", type=str, help="Image URL")
    stage_parser.add_argument(
        "--json",
        action="store_true",
        help="Output staged path as JSON",
    )

    # Command: cover
    cover_parser = subparsers.add_parser("cover", help="Search studio cover art (iTunes + Deezer + Cover Art Archive)")
    cover_parser.add_argument("query", type=str, nargs="?", default="", help="Free-text query (or title when --artist given)")
    cover_parser.add_argument("--artist", type=str, default="", help="Artist name for scored search")
    cover_parser.add_argument("--title", type=str, default="", help="Track title for scored search")
    cover_parser.add_argument("-l", "--limit", type=int, default=16, help="Number of results (default: 16)")
    cover_parser.add_argument(
        "--source",
        choices=["auto", "all", "itunes", "deezer", "caa", "web"],
        default="auto",
        help="Providers to query (default: auto = itunes+deezer+caa; all = includes web)",
    )
    cover_parser.add_argument("--prefetch", action="store_true", help="Download candidate images into local disk cache")
    cover_parser.add_argument("--json", action="store_true", help="Output results as JSON")

    # Command: tag-genre (hand-curated sidecar genres for the TUI picker)
    genre_parser = subparsers.add_parser("tag-genre", help="Set sidecar genres for a file")
    genre_parser.add_argument("file", type=str, help="Audio file path")
    genre_parser.add_argument("genres", type=str, nargs="?", default="",
                              help="Comma-separated genres (empty clears the entry)")

    # Command: fill-genres (one-shot iTunes backfill for an existing library)
    fill_parser = subparsers.add_parser("fill-genres", help="Backfill sidecar genres from iTunes")
    fill_parser.add_argument("directory", type=str, help="Music directory to scan")
    fill_parser.add_argument("--limit", type=int, default=None, help="Max files to query (default: all)")
    fill_parser.add_argument("--json", action="store_true", help="Output stats as JSON")

    # Command: lyric-search
    lyric_search_parser = subparsers.add_parser("lyric-search", help="Search synced lyrics via LRCLIB (with Groq Whisper AI fallback)")
    lyric_search_parser.add_argument("--artist", type=str, required=True, help="Artist name")
    lyric_search_parser.add_argument("--title", type=str, required=True, help="Track title")
    lyric_search_parser.add_argument("--album", type=str, default="", help="Album name (optional)")
    lyric_search_parser.add_argument("--duration", type=int, default=0, help="Track duration in seconds (optional)")
    lyric_search_parser.add_argument("--audio-file", type=str, default="", help="Local audio file for Groq Whisper AI fallback")
    lyric_search_parser.add_argument("--ai", action="store_true", help="Force Groq Whisper AI lyric generation from --audio-file")
    lyric_search_parser.add_argument("--json", action="store_true", help="Output results as JSON")

    # Command: lyric-verify
    lyric_verify_parser = subparsers.add_parser("lyric-verify", help="Verify whether existing .lrc lyrics match audio vocals")
    lyric_verify_parser.add_argument("--audio-file", type=str, required=True, help="Audio file path")
    lyric_verify_parser.add_argument("--lrc-file", type=str, default="", help="Optional explicit .lrc file path")
    lyric_verify_parser.add_argument("--json", action="store_true", help="Output result as JSON")

    # Command: fill-lyrics (auto-generate or verify+fix .lrc files for all existing songs in a folder)
    fill_lrc_parser = subparsers.add_parser("fill-lyrics", help="Verify existing .lrc files and generate missing/mismatched ones via LRCLIB + Groq Whisper AI")
    fill_lrc_parser.add_argument("directory", type=str, nargs="?", default=None, help="Music directory to scan (default: configured music_folder or ~/Music)")
    fill_lrc_parser.add_argument("--ai-only", action="store_true", help="Always use Groq Whisper AI even if LRCLIB has results")
    fill_lrc_parser.add_argument("--verify", action="store_true", help="Also verify existing .lrc files against audio and fix wrong ones")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(1)

    if args.command == "search":
        try:
            from .search import search_music
            results = search_music(args.query, limit=args.limit, source=args.source)
            if args.json:
                print(json.dumps({"type": "search_results", "items": results}), flush=True)
            else:
                for i, r in enumerate(results, 1):
                    print(
                        f"{i}. [{r.get('source', '?')}] {r['artist']} - {r['title']} "
                        f"[{r['duration']}] ({r['uploader']})"
                    )
                    if r.get("url"):
                        print(f"   URL: {r['url']}")
                    if r.get("album"):
                        print(f"   Album: {r['album']}")
        except Exception as e:
            if args.json:
                print(json.dumps({"type": "error", "message": str(e)}), flush=True)
            else:
                print(f"error: {e}", file=sys.stderr)
            sys.exit(1)

    elif args.command == "resolve":
        try:
            from .search import resolve
            info = resolve(
                args.artist,
                args.title,
                album=args.album,
                duration=args.duration or None,
                source=args.source,
                cover_url=args.cover_url,
                year=args.year,
            )
            if args.json:
                print(json.dumps({"type": "info", "data": info}), flush=True)
            else:
                print(f"Title:    {info['title']}")
                print(f"Artist:   {info['artist']}")
                print(f"Album:    {info.get('album', '')}")
                print(f"Duration: {info['duration']}")
                print(f"URL:      {info['url']}")
                print(f"Lyrics:   {'yes' if info.get('has_lyrics') else 'no'}")
        except Exception as e:
            if args.json:
                print(json.dumps({"type": "error", "message": str(e)}), flush=True)
            else:
                print(f"error: {e}", file=sys.stderr)
            sys.exit(1)

    elif args.command == "info":
        try:
            info = get_video_info(args.url)
            if args.json:
                print(json.dumps({"type": "info", "data": info}), flush=True)
            else:
                print(f"Title:    {info['title']}")
                print(f"Artist:   {info['artist']}")
                print(f"Duration: {info['duration']}")
        except Exception as e:
            if args.json:
                print(json.dumps({"type": "error", "message": str(e)}), flush=True)
            else:
                print(f"error: {e}", file=sys.stderr)
            sys.exit(1)

    elif args.command == "probe-lyrics":
        try:
            meta_probe = None
            if args.artist or args.title:
                from . import lrclib
                meta_probe = lrclib.probe(
                    args.artist, args.title, args.album, args.duration or None
                )

            # LRCLIB synced lyrics win in the download pipeline, so skip the
            # slower YouTube probe when they are already available.
            video_probe = None
            if args.url and not (meta_probe and meta_probe.get("has_manual")):
                video_probe = probe_lyrics(args.url)

            if meta_probe and meta_probe.get("has_manual"):
                probe = {
                    "source": "lrclib",
                    "has_manual": True,
                    "has_auto": bool(meta_probe.get("has_auto")),
                    "langs": [],
                }
            elif video_probe and (video_probe.get("has_manual") or video_probe.get("has_auto")):
                probe = {"source": "youtube", **video_probe}
            elif meta_probe and meta_probe.get("has_auto"):
                probe = {
                    "source": "lrclib",
                    "has_manual": False,
                    "has_auto": True,
                    "langs": [],
                }
            else:
                probe = {
                    "source": "",
                    "has_manual": False,
                    "has_auto": False,
                    "langs": [],
                }

            if args.json:
                print(json.dumps({"type": "lyrics_probe", **probe}), flush=True)
            else:
                if probe["source"] == "lrclib":
                    print("Source: LRCLIB")
                    print(f"Synced lyrics: {'yes' if probe['has_manual'] else 'no'}")
                    print(f"Plain lyrics:  {'yes' if probe['has_auto'] else 'no'}")
                elif probe["source"] == "youtube":
                    print("Source: YouTube captions")
                    print(f"Manual subtitles: {'yes' if probe['has_manual'] else 'no'}")
                    print(f"Auto subtitles:   {'yes' if probe['has_auto'] else 'no'}")
                else:
                    print("No lyrics found")
        except Exception as e:
            if args.json:
                print(json.dumps({"type": "error", "message": str(e)}), flush=True)
            else:
                print(f"error: {e}", file=sys.stderr)
            sys.exit(1)

    elif args.command == "stage-image":
        try:
            from .artwork.images import stage_remote_image
            path = stage_remote_image(args.url)
            if args.json:
                if path:
                    print(json.dumps({"type": "staged", "path": path}), flush=True)
                else:
                    print(json.dumps({"type": "error", "message": "Download failed"}), flush=True)
            else:
                if path:
                    print(path)
                else:
                    print("error: download failed", file=sys.stderr)
                    sys.exit(1)
        except Exception as e:
            if args.json:
                print(json.dumps({"type": "error", "message": str(e)}), flush=True)
            else:
                print(f"error: {e}", file=sys.stderr)
            sys.exit(1)
    elif args.command == "cover":
        from .artwork import search_candidates, prefetch_candidates
        try:
            artist = args.artist or ""
            title = args.title or args.query
            if not artist and not args.title and " - " in (args.query or ""):
                parts = args.query.split(" - ", 1)
                artist = parts[0].strip()
                title = parts[1].strip()
            if not artist and not title:
                print(json.dumps({"type": "error", "message": "Provide a query or --artist/--title"}), flush=True) \
                    if args.json else print("error: empty query", file=sys.stderr)
                sys.exit(1)
            if args.source == "auto":
                sources = ("itunes", "deezer", "caa")
            elif args.source == "all":
                sources = ("itunes", "deezer", "caa", "web")
            else:
                sources = (args.source,)
            results = search_candidates(artist, title or args.query, sources=sources, limit=args.limit)
            if args.prefetch or args.json:
                prefetch_candidates(results)
            if args.json:
                print(json.dumps({"type": "cover_results", "items": results}), flush=True)
            else:
                if not results:
                    print("No cover art found.")
                for i, r in enumerate(results, 1):
                    yr = f" ({r['year']})" if r.get('year') else ""
                    print(f"{i}. [{r['source']}:{r.get('score')}] {r['artist']} - {r['title']} [Album: {r['album']}]{yr}")
                    print(f"   Cover: {r['cover_url']}")
        except Exception as e:
            if args.json:
                print(json.dumps({"type": "error", "message": str(e)}), flush=True)
            else:
                print(f"error: {e}", file=sys.stderr)
            sys.exit(1)

    elif args.command == "lyric-search":
        try:
            from .lyric import (
                search_lrclib,
                transcribe_lyrics_groq,
                normalize_lyric_query,
                find_local_audio_file,
                verify_lrc_with_audio,
            )
            raw_artist = (args.artist or "").strip()
            raw_title = (args.title or "").strip()
            force_ai = bool(args.ai)
            if raw_artist.lower().startswith("ai:"):
                force_ai = True
                raw_artist = raw_artist[3:].strip()
            if raw_title.lower().startswith("ai:"):
                force_ai = True
                raw_title = raw_title[3:].strip()

            items = []
            if not force_ai:
                items = search_lrclib(
                    artist=raw_artist,
                    title=raw_title,
                    album=args.album,
                    duration=args.duration,
                    limit=10,
                )

            audio_file = args.audio_file
            if not audio_file or not os.path.isfile(audio_file):
                audio_file = find_local_audio_file(raw_artist, raw_title) or ""

            # Cross-verify the top LRCLIB synced candidate against actual vocals in audio_file
            top_synced = next((it for it in items if it.get("synced") and it.get("lyrics")), None)
            lrclib_verified = False
            if top_synced and audio_file and os.path.isfile(audio_file) and not force_ai:
                is_ok, conf, _ = verify_lrc_with_audio(audio_file, top_synced["lyrics"])
                if is_ok:
                    lrclib_verified = True
                    old_alb = top_synced.get("album") or ""
                    top_synced["album"] = f"{old_alb} [✓ {int(conf * 100)}%]" if old_alb else f"✓ {int(conf * 100)}% Match"
                else:
                    old_alb = top_synced.get("album") or ""
                    top_synced["album"] = f"{old_alb} [⚠️ Mismatch]" if old_alb else "⚠️ Audio Mismatch"

            if (force_ai or not lrclib_verified) and audio_file and os.path.isfile(audio_file):
                clean_a, clean_t = normalize_lyric_query(raw_artist, raw_title)
                ai_lrc = transcribe_lyrics_groq(
                    audio_path=audio_file,
                    artist=clean_a,
                    title=clean_t,
                )
                if ai_lrc and ai_lrc.strip():
                    ai_item = {
                        "artist": clean_a or raw_artist,
                        "title": clean_t or raw_title,
                        "album": "Groq Whisper AI ✓",
                        "duration": args.duration or 0,
                        "lyrics": ai_lrc,
                        "synced": True,
                    }
                    items.insert(0, ai_item)

            best = items[0] if items else {}
            result = {
                "type": "lyric_search",
                "artist": best.get("artist", args.artist),
                "title": best.get("title", args.title),
                "album": best.get("album", ""),
                "duration": best.get("duration", 0),
                "lyrics": best.get("lyrics", ""),
                "synced": bool(best.get("synced", False)),
                "items": items,
            }
            if args.json:
                print(json.dumps(result), flush=True)
            else:
                if result["lyrics"]:
                    print(f"Found lyrics for {result['artist']} - {result['title']}")
                    print(f"Synced: {'yes' if result['synced'] else 'no'}")
                    print(f"Duration: {result['duration']}s")
                    print("---")
                    print(result["lyrics"])
                else:
                    print("No lyrics found.")
        except Exception as e:
            if args.json:
                print(json.dumps({"type": "error", "message": str(e)}), flush=True)
            else:
                print(f"error: {e}", file=sys.stderr)
            sys.exit(1)

    elif args.command == "lyric-verify":
        from .lyric import verify_existing_lrc
        try:
            res = verify_existing_lrc(
                audio_path=args.audio_file,
                lrc_path=args.lrc_file or None,
            )
            if args.json:
                print(json.dumps(res), flush=True)
            else:
                pct = int(round(res["confidence"] * 100))
                print(f"Status: {res['status']} ({pct}%) | Timing: {'yes' if res['has_timing'] else 'no'}")
                if res["lrc_path"]:
                    print(f"File:   {res['lrc_path']}")
                if res["heard"]:
                    print(f"Heard:  {res['heard']}")
        except Exception as e:
            if args.json:
                print(json.dumps({"type": "error", "message": str(e)}), flush=True)
            else:
                print(f"error: {e}", file=sys.stderr)
            sys.exit(1)

    elif args.command == "fill-lyrics":
        from .lyric import (
            search_lrclib,
            transcribe_lyrics_groq,
            normalize_lyric_query,
            verify_lrc_with_audio,
            configured_music_roots,
        )
        raw_dir = args.directory or (configured_music_roots() or [os.path.expanduser("~/Music")])[0]
        target_dir = os.path.abspath(os.path.expanduser(raw_dir))
        if not os.path.isdir(target_dir):
            print(f"error: directory not found: {target_dir}", file=sys.stderr)
            sys.exit(1)
        total = 0
        created = 0
        for root, _, files in os.walk(target_dir):
            for fn in sorted(files):
                if not fn.lower().endswith(".mp3"):
                    continue
                mp3_p = os.path.join(root, fn)
                lrc_p = os.path.splitext(mp3_p)[0] + ".lrc"
                if os.path.isfile(lrc_p):
                    if not args.verify:
                        continue
                    try:
                        with open(lrc_p, "r", encoding="utf-8", errors="ignore") as f:
                            existing_lrc = f.read()
                        is_ok, conf, reason = verify_lrc_with_audio(mp3_p, existing_lrc)
                        has_timing = bool(re.search(r"\[\d{2}:\d{2}", existing_lrc))
                        if is_ok and has_timing:
                            print(f"[OK {int(conf * 100)}%] {fn}", flush=True)
                            continue
                        if is_ok and not has_timing:
                            print(f"[PLAIN {int(conf * 100)}%, no timing] {fn} -> Upgrading to synced lyrics...", flush=True)
                        else:
                            print(f"[MISMATCH {int(conf * 100)}%] {fn} ({reason}) -> Regenerating lyrics...", flush=True)
                    except OSError as exc:
                        print(f"[READ ERR] {fn}: {exc}", flush=True)
                        continue
                total += 1
                stem = os.path.splitext(fn)[0]
                artist, title = normalize_lyric_query(stem, stem)
                print(f"[{total}] Generating .lrc for: {fn} ...", flush=True)
                lrc_text = ""
                source = ""
                if not args.ai_only:
                    hits = search_lrclib(artist, title, limit=3)
                    synced = next((h for h in hits if h.get("synced") and h.get("lyrics")), None)
                    if synced:
                        is_ok, conf, _ = verify_lrc_with_audio(mp3_p, synced["lyrics"])
                        if is_ok:
                            lrc_text = synced["lyrics"]
                            source = f"LRCLIB ✓ {int(conf * 100)}%"
                        else:
                            print(f"  -> Rejected mismatched LRCLIB lyrics ({int(conf * 100)}% overlap)", flush=True)
                if not lrc_text:
                    ai_res = transcribe_lyrics_groq(mp3_p, artist=artist, title=title)
                    if ai_res:
                        lrc_text = ai_res
                        source = "Groq Whisper AI"
                if lrc_text:
                    with open(lrc_p, "w", encoding="utf-8") as f:
                        f.write(lrc_text)
                    created += 1
                    print(f"  -> Saved {os.path.basename(lrc_p)} [{source}]", flush=True)
                else:
                    print("  -> No lyrics found/generated.", flush=True)
        print(f"Done: updated/generated {created}/{total} .lrc files.")

    elif args.command == "tag-genre":
        from .genres import set_genres
        try:
            genres = [g.strip() for g in args.genres.split(",") if g.strip()]
            saved = set_genres(args.file, genres)
            print(json.dumps({"type": "genres", "file": args.file, "genres": saved}), flush=True)
        except Exception as e:
            print(json.dumps({"type": "error", "message": str(e)}), flush=True)
            sys.exit(1)

    elif args.command == "fill-genres":
        from .genres import backfill_directory
        try:
            def show_progress(path, genre):
                name = os.path.basename(path)
                print(f"  {'✓' if genre else '·'} {name}" + (f" [{genre}]" if genre else ""), flush=True)
            if not args.json:
                print(f"Backfilling genres in {args.directory} ...")
            stats = backfill_directory(args.directory, limit=args.limit,
                                       progress=None if args.json else show_progress)
            if args.json:
                print(json.dumps({"type": "genre_stats", **stats}), flush=True)
            else:
                print(f"Done: {stats['filled']} filled, {stats['skipped']} skipped, "
                      f"{stats['missed']} missed ({stats['scanned']} scanned)")
        except Exception as e:
            print(json.dumps({"type": "error", "message": str(e)}), flush=True) \
                if args.json else print(f"error: {e}", file=sys.stderr)
            sys.exit(1)

    elif args.command == "get":
        try:
            crop_focus = None
            if args.crop_focus:
                try:
                    x_str, y_str = args.crop_focus.replace(",", " ").split()
                    crop_focus = (float(x_str), float(y_str))
                except (ValueError, AttributeError):
                    print("warning: ignoring invalid --crop-focus (want 'x y' in 0..1)", file=sys.stderr)
            pipeline = LyraPipeline(
                output_dir=args.output,
                cover_mode=args.cover_mode,
                json_mode=args.json,
                cover_source=args.cover_source,
                cover_url=args.cover_url,
                custom_cover=args.cover or args.cover_url,
                no_cover_search=args.no_cover_search,
                crop_focus=crop_focus,
            )
            pipeline.process_url(
                args.url,
                name=args.name,
                title=args.title,
                album=args.album,
                singer=args.singer,
                genre=args.genre,
                no_lyrics=args.no_lyrics,
                no_auto_lyrics=args.no_auto_lyrics,
                lyrics_lang=args.lyrics_lang,
            )
        except KeyboardInterrupt:
            if args.json:
                print(json.dumps({"type": "error", "message": "Aborted by user"}), flush=True)
            else:
                print("\nAborted.", file=sys.stderr)
            sys.exit(130)
        except Exception as e:
            if args.json:
                print(json.dumps({"type": "error", "message": str(e)}), flush=True)
            else:
                print(f"error: {e}", file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    main()
