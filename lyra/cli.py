import sys
import argparse
from .core import LyraPipeline


def main():
    parser = argparse.ArgumentParser(
        prog="lyra",
        description="Download audio, generate square cover art, and extract synced lyrics.",
    )
    subparsers = parser.add_subparsers(dest="command")

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
        "-n" ,"--name" ,
        type=str,
        default = None ,
        help="Name the video (default : <Artist> - <Title>)",
    )
    get_parser.add_argument(
        "-a" ,"--album" ,
        type=str ,
        default = None,
        help="Album of the song (default : single track)",
    )
    get_parser.add_argument(
        "-s","--singer" ,
        type=str,
        default=None,
        help="Author of the song (default : name of album)"
    )

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(1)

    if args.command == "get":
        try:
            pipeline = LyraPipeline(output_dir=args.output, cover_mode=args.cover_mode)
            pipeline.process_url(args.url , name = args.name , album = args.album,singer = args.singer)
        except KeyboardInterrupt:
            print("\nAborted.", file=sys.stderr)
            sys.exit(130)
        except Exception as e:
            print(f"error: {e}", file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    main()
