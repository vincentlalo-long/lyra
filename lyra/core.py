import os
import shutil
import tempfile
import glob
from typing import Dict, Any

import yt_dlp

from .utils import clean_title_and_artist, sanitize_filename
from .lyric import vtt_to_lrc, fetch_best_vtt
from .image import process_cover_art
from .tagger import tag_audio_file


class LyraPipeline:
    def __init__(self, output_dir: str = "./output", cover_mode: str = "blur_pad"):
        self.output_dir = os.path.abspath(os.path.expanduser(output_dir))
        self.cover_mode = cover_mode
        os.makedirs(self.output_dir, exist_ok=True)

    def process_url(self, url: str , name : str |None = None , album : str |None = None ,singer : str |None = None) -> Dict[str, Any]:
        print(f"Fetching: {url}")

        with tempfile.TemporaryDirectory(prefix="lyra_") as temp_dir:
            out_template = os.path.join(temp_dir, "media.%(ext)s")

            # Extract audio and thumbnail only
            ydl_opts = {
                "format": "bestaudio/best",
                "outtmpl": out_template,
                "writethumbnail": True,
                "noplaylist": True,
                "quiet": True,
                "no_warnings": True,
                "postprocessors": [
                    {
                        "key": "FFmpegExtractAudio",
                        "preferredcodec": "mp3",
                        "preferredquality": "192",
                    },
                ],
            }

            if shutil.which("node"):
                ydl_opts["js_runtimes"] = {"node": {}}

            with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                info = ydl.extract_info(url, download=True)

            raw_title = info.get("title", "Unknown Title")
            uploader = info.get("uploader", "")
            artist, title = clean_title_and_artist(raw_title, uploader)
            if singer:
                artist = singer.strip()
            print(f"Track:    {artist} - {title}")

            # Locate converted audio
            mp3_candidates = glob.glob(os.path.join(temp_dir, "*.mp3"))
            if not mp3_candidates:
                raise FileNotFoundError("Audio extraction failed; no .mp3 generated.")
            raw_mp3_path = mp3_candidates[0]

            # Fetch subtitles for 'en' or 'vi' if available; early exit if not found
            lrc_content = ""
            vtt_text = fetch_best_vtt(info, target_langs=["en", "vi"])
            if vtt_text:
                try:
                    lrc_content = vtt_to_lrc(vtt_text)
                except Exception:
                    lrc_content = ""

            # Process 1:1 cover art
            processed_cover_path = os.path.join(temp_dir, "cover_1x1.jpg")
            has_cover = False

            img_candidates = []
            for ext in ("*.webp", "*.jpg", "*.jpeg", "*.png"):
                img_candidates.extend(glob.glob(os.path.join(temp_dir, ext)))

            if img_candidates:
                has_cover = process_cover_art(
                    img_candidates[0],
                    processed_cover_path,
                    mode=self.cover_mode,
                    target_size=1000
                )

            # Embed metadata & cover art
            tag_audio_file(
                file_path=raw_mp3_path,
                title=title,
                artist=artist,
                album = album if album else f"{title} - Single",
                cover_path=processed_cover_path if has_cover else None,
                lrc_text=lrc_content if lrc_content else None,
            )

            # Export to target directory
            if album : 
                clean_album = sanitize_filename(album)
                target_dir = os.path.join(self.output_dir, clean_album)
                os.makedirs(target_dir , exist_ok=True)
            else :
                target_dir = self.output_dir
            if(name) :
                clean_name = os.path.splitext(name)[0]
                base_filename = sanitize_filename(clean_name)
            else :
                base_filename = sanitize_filename(f"{artist} - {title}")
            dest_mp3 = os.path.join(target_dir, f"{base_filename}.mp3")
            dest_lrc = os.path.join(target_dir, f"{base_filename}.lrc")

            shutil.copy2(raw_mp3_path, dest_mp3)
            print(f"Audio:    {dest_mp3}")

            if lrc_content:
                with open(dest_lrc, "w", encoding="utf-8") as f:
                    f.write(lrc_content)
                print(f"Lyrics:   {dest_lrc}")
            else:
                print("Lyrics:   none found")

            return {
                "artist": artist,
                "title": title,
                "audio_path": dest_mp3,
                "lyric_path": dest_lrc if lrc_content else None,
                "has_cover": has_cover,
            }
