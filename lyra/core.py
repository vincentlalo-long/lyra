import os
import shutil
import tempfile
import glob
import json
from typing import Dict, Any, List, Optional

import yt_dlp

from .utils import clean_title_and_artist, sanitize_filename
from .lyric import vtt_to_lrc, fetch_best_vtt, fetch_all_vtt
from .image import process_cover_art
from .tagger import tag_audio_file


def search_youtube(query: str, limit: int = 5) -> List[Dict[str, Any]]:
    ydl_opts = {
        "extract_flat": "in_playlist",
        "quiet": True,
        "no_warnings": True,
    }
    with yt_dlp.YoutubeDL(ydl_opts) as ydl:
        res = ydl.extract_info(f"ytsearch{limit}:{query}", download=False)
        entries = res.get("entries", []) if res else []
        results = []
        for entry in entries:
            video_id = entry.get("id", "")
            raw_title = entry.get("title", "")
            uploader = entry.get("uploader", "")
            artist, title = clean_title_and_artist(raw_title, uploader)
            duration_sec = entry.get("duration") or 0
            mins = int(duration_sec // 60)
            secs = int(duration_sec % 60)
            results.append({
                "id": video_id,
                "url": f"https://www.youtube.com/watch?v={video_id}",
                "raw_title": raw_title,
                "title": title,
                "artist": artist,
                "uploader": uploader,
                "duration": f"{mins:02}:{secs:02}",
            })
        return results


def get_video_info(url: str) -> Dict[str, Any]:
    ydl_opts = {
        "extract_flat": True,
        "quiet": True,
        "no_warnings": True,
    }
    with yt_dlp.YoutubeDL(ydl_opts) as ydl:
        info = ydl.extract_info(url, download=False)
        raw_title = info.get("title", "")
        uploader = info.get("uploader", "")
        artist, title = clean_title_and_artist(raw_title, uploader)
        duration_sec = info.get("duration") or 0
        mins = int(duration_sec // 60)
        secs = int(duration_sec % 60)
        return {
            "id": info.get("id", ""),
            "url": url,
            "raw_title": raw_title,
            "title": title,
            "artist": artist,
            "uploader": uploader,
            "duration": f"{mins:02}:{secs:02}",
        }


class LyraPipeline:
    def __init__(self, output_dir: str = "./output", cover_mode: str = "blur_pad", json_mode: bool = False):
        self.output_dir = os.path.abspath(os.path.expanduser(output_dir))
        self.cover_mode = cover_mode
        self.json_mode = json_mode
        os.makedirs(self.output_dir, exist_ok=True)

    def _emit(self, data: dict):
        if self.json_mode:
            print(json.dumps(data), flush=True)

    def process_url(
        self,
        url: str,
        name: Optional[str] = None,
        album: Optional[str] = None,
        singer: Optional[str] = None,
        no_lyrics: bool = False,
        no_auto_lyrics: bool = False,
    ) -> Dict[str, Any]:
        if not self.json_mode:
            print(f"Fetching: {url}")
        else:
            self._emit({"type": "status", "stage": "Connecting to YouTube..."})

        with tempfile.TemporaryDirectory(prefix="lyra_") as temp_dir:
            out_template = os.path.join(temp_dir, "media.%(ext)s")

            def ydl_progress_hook(d):
                if d.get("status") == "downloading":
                    total = d.get("total_bytes") or d.get("total_bytes_estimate") or 0
                    downloaded = d.get("downloaded_bytes", 0)
                    pct = round((downloaded / total * 100), 1) if total > 0 else 0
                    speed_bytes = d.get("speed")
                    speed = f"{round(speed_bytes / 1024 / 1024, 1)} MB/s" if speed_bytes else ""
                    eta_sec = d.get("eta")
                    eta = f"{eta_sec}s" if eta_sec else ""
                    self._emit({
                        "type": "progress",
                        "percent": pct,
                        "speed": speed,
                        "eta": eta,
                        "stage": "Downloading audio stream...",
                    })
                elif d.get("status") == "finished":
                    self._emit({"type": "status", "stage": "Converting to MP3 audio..."})

            ydl_opts = {
                "format": "bestaudio/best",
                "outtmpl": out_template,
                "writethumbnail": True,
                "noplaylist": True,
                "quiet": True,
                "no_warnings": True,
                "progress_hooks": [ydl_progress_hook] if self.json_mode else [],
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
            if not self.json_mode:
                print(f"Track:    {artist} - {title}")

            # Locate converted audio
            mp3_candidates = glob.glob(os.path.join(temp_dir, "*.mp3"))
            if not mp3_candidates:
                raise FileNotFoundError("Audio extraction failed; no .mp3 generated.")
            raw_mp3_path = mp3_candidates[0]

            # Fetch subtitles for 'vi' or 'en' (and all languages if creator uploaded multiple)
            lrc_map: Dict[str, str] = {}
            primary_lrc = ""
            if not no_lyrics:
                self._emit({"type": "status", "stage": "Extracting synced lyrics (.lrc)..."})
                try:
                    vtt_dict = fetch_all_vtt(
                        info,
                        target_langs=["vi", "en"],
                        allow_auto=not no_auto_lyrics,
                        download_all=True,
                    )
                    for lang, vtt_text in vtt_dict.items():
                        parsed = vtt_to_lrc(vtt_text)
                        if parsed.strip():
                            lrc_map[lang] = parsed

                    if lrc_map:
                        # Prioritize 'vi' first, then 'en', then first available language
                        primary_lang = None
                        for candidate in ["vi", "en"]:
                            for k in lrc_map.keys():
                                if k == candidate or k.startswith(f"{candidate}-"):
                                    primary_lang = k
                                    break
                            if primary_lang:
                                break
                        if not primary_lang:
                            primary_lang = next(iter(lrc_map.keys()))
                        primary_lrc = lrc_map[primary_lang]
                except Exception as e:
                    if not self.json_mode:
                        print(f"Warning: Failed to extract lyrics: {e}")

            # Process 1:1 cover art
            self._emit({"type": "status", "stage": "Generating 1:1 square cover art..."})
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
                    target_size=1000,
                )

            # Embed metadata & cover art
            self._emit({"type": "status", "stage": "Tagging ID3v2 metadata..."})
            tag_audio_file(
                file_path=raw_mp3_path,
                title=title,
                artist=artist,
                album=album if album else f"{title} - Single",
                cover_path=processed_cover_path if has_cover else None,
                lrc_text=primary_lrc if primary_lrc else None,
            )

            # Export to target directory
            if album:
                clean_album = sanitize_filename(album)
                target_dir = os.path.join(self.output_dir, clean_album)
                os.makedirs(target_dir, exist_ok=True)
            else:
                target_dir = self.output_dir

            if name:
                clean_name = os.path.splitext(name)[0]
                base_filename = sanitize_filename(clean_name)
            else:
                base_filename = sanitize_filename(f"{artist} - {title}")

            dest_mp3 = os.path.join(target_dir, f"{base_filename}.mp3")
            dest_lrc = os.path.join(target_dir, f"{base_filename}.lrc")

            shutil.copy2(raw_mp3_path, dest_mp3)
            if not self.json_mode:
                print(f"Audio:    {dest_mp3}")

            saved_lrc_paths = []
            if primary_lrc:
                # Save standard <base_filename>.lrc for default music player detection
                with open(dest_lrc, "w", encoding="utf-8") as f:
                    f.write(primary_lrc)
                saved_lrc_paths.append(dest_lrc)
                if not self.json_mode:
                    print(f"Lyrics:   {dest_lrc}")

                # If multiple subtitles exist, also save <base_filename>.<lang>.lrc
                if len(lrc_map) > 1:
                    for lang, content in lrc_map.items():
                        lang_lrc = os.path.join(target_dir, f"{base_filename}.{lang}.lrc")
                        with open(lang_lrc, "w", encoding="utf-8") as f:
                            f.write(content)
                        saved_lrc_paths.append(lang_lrc)
                        if not self.json_mode:
                            print(f"Lyrics ({lang}): {lang_lrc}")
            else:
                if not self.json_mode:
                    print("Lyrics:   disabled (--no-lyrics)" if no_lyrics else "Lyrics:   none found")

            result = {
                "type": "done",
                "artist": artist,
                "title": title,
                "audio_path": dest_mp3,
                "lyric_path": dest_lrc if primary_lrc else None,
                "all_lyric_paths": saved_lrc_paths,
                "has_cover": has_cover,
            }
            self._emit(result)
            return result
