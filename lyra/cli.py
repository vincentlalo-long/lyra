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

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(1)

    if args.command == "get":
        try:
            pipeline = LyraPipeline(output_dir=args.output, cover_mode=args.cover_mode)
            pipeline.process_url(args.url)
        except KeyboardInterrupt:
            print("\nAborted.", file=sys.stderr)
            sys.exit(130)
        except Exception as e:
            print(f"error: {e}", file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    main()
