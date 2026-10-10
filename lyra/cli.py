import os
import sys
import json
import argparse
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
        "--json",
        action="store_true",
        help="Output line-by-line JSON progress events for GUI/TUI integration",
    )

    # Command: search
    search_parser = subparsers.add_parser("search", help="Search YouTube for tracks")
    search_parser.add_argument("query", type=str, help="Search query")
    search_parser.add_argument(
        "-l", "--limit",
        type=int,
        default=15,
        help="Number of results (default: 15)",
    )
    search_parser.add_argument(
        "--json",
        action="store_true",
        help="Output results as JSON",
    )

    # Command: info
    info_parser = subparsers.add_parser("info", help="Get track info from URL")
    info_parser.add_argument("url", type=str, help="Video URL")
    info_parser.add_argument(
        "--json",
        action="store_true",
        help="Output info as JSON",
    )

    # Command: probe-lyrics
    probe_parser = subparsers.add_parser("probe-lyrics", help="Check subtitle availability for a video")
    probe_parser.add_argument("url", type=str, help="Video URL")
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

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(1)

    if args.command == "search":
        try:
            results = search_youtube(args.query, limit=args.limit)
            if args.json:
                print(json.dumps({"type": "search_results", "items": results}), flush=True)
            else:
                for i, r in enumerate(results, 1):
                    print(f"{i}. {r['artist']} - {r['title']} [{r['duration']}] ({r['uploader']})")
                    print(f"   URL: {r['url']}")
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
            probe = probe_lyrics(args.url)
            if args.json:
                print(json.dumps({"type": "lyrics_probe", **probe}), flush=True)
            else:
                print(f"Manual subs: {'yes' if probe['has_manual'] else 'no'}")
                print(f"Auto subs:   {'yes' if probe['has_auto'] else 'no'}")
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
