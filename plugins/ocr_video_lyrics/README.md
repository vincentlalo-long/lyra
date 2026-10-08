# OCR Video HardCode Lyric to LRC

A planned Lyra extension to extract hardcoded, non-selectable subtitles from music videos (e.g., YouTube MVs) and convert them into time-synchronized `.lrc` lyric files.

## Status: Coming Soon / In Development

Author: [@vincentlalo-long](https://github.com/vincentlalo-long)

## Pipeline Overview

1. Sample video frames around subtitle bounding boxes (bottom thirds).
2. Frame difference & motion deduplication to find subtitle transition timestamps.
3. Optical Character Recognition (OCR) engine for multi-language text recognition.
4. Output standard time-tagged `.lrc` format (`[mm:ss.xx] Lyric text`).
