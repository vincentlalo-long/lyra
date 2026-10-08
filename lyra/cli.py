import sys
import json
import argparse
from .core import LyraPipeline, search_youtube, get_video_info


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
        choices=["blur_pad", "center_crop"],
        default="blur_pad",
        help="Cover art mode: blur_pad (default) or center_crop",
    )
    get_parser.add_argument(
        "-n", "--name",
        type=str,
        default=None,
        help="Name of the file (default: <Artist> - <Title>)",
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
        default=5,
        help="Number of results (default: 5)",
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

    elif args.command == "get":
        try:
            pipeline = LyraPipeline(
                output_dir=args.output,
                cover_mode=args.cover_mode,
                json_mode=args.json,
            )
            pipeline.process_url(
                args.url,
                name=args.name,
                album=args.album,
                singer=args.singer,
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
