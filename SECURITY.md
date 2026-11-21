# Security Policy

## Supported versions

| Version | Supported |
|---|---|
| 0.3.x (latest release) | Yes |
| < 0.3 | No |

Only the latest published release receives security fixes.

## Reporting a vulnerability

Do **not** open a public issue for a security problem. Report it privately via
GitHub Security Advisories:

<https://github.com/vincentlalo-long/lyra/security/advisories/new>

Please include:

- Lyra version (`lyra --version`) and install method (release tarball, cargo, distro package).
- Operating system and terminal.
- A minimal reproduction or proof of concept.
- Impact you believe the issue has.

Never include Spotify credentials (`SPOTIFY_CLIENT_ID`, `SPOTIFY_CLIENT_SECRET`,
`~/.config/lyra/spotify.json`), tokens, or personal file paths in a report.

Reports are handled on a best-effort basis. Please allow time for a fix before
public disclosure.

## Scope and known risks

Lyra intentionally executes external tools and third-party code:

- **Python helper pipeline**: the Full TUI invokes `python3 main.py` with
  `yt-dlp`, `pillow`, and `mutagen`. Keep these packages up to date; they handle
  untrusted remote media and metadata.
- **Community plugins**: plugins from `registry.json` run as ordinary local
  Python with your user privileges. They are not sandboxed or audited. Review a
  plugin's source before installing it, and install only from authors you trust.
- **Self-update**: downloads are SHA-256 verified against the release manifest
  before replacement, and only tarball installs in writable user directories are
  updated. Package-manager installs are never overwritten.

Vulnerabilities in third-party dependencies should be reported upstream, but a
note here is welcome if Lyra needs a version bump.
