# Eightbit Studio · Browser edition

Build, edit and play BASIC programmes for Dragon 32, Commodore 64 PAL, ZX Spectrum and ZX81 in your browser.

[Open Studio](https://eightbitstudio.com/studio/) · [Matching source release](https://github.com/robtyers/eightbit-studio-browser/releases/tag/browser-0.5.2-20261010.1) · [Build guide](docs/browser-edition.md)

## Source and downloads

This repository contains the browser edition and its required shared source. The matching release supplies complete browser-source.tar.gz and mame0289.tar.gz archives, build-receipt.json and SHA256SUMS. Download the MAME archive into .cache/mame-web/ before building Sinclair emulation. The browser archive also contains pinned XRoar/chips/SDL sources, JavaScript package sources and vendored Rust dependencies. No system ROMs are supplied; add your own supported firmware to play.

See docs/browser-edition.md for exact tool versions, build commands, verification and local preview instructions. SOURCE-MANIFEST.json identifies this source snapshot. Each deployed version links to its corresponding tagged release.

## Licences

The browser application is **GPL-3.0-or-later**; see [COPYING](COPYING). Reused Eightbit Studio libraries/components retain [Apache-2.0](LICENSE). Third-party attribution and licence texts remain in [NOTICE.txt](NOTICE.txt), vendor/ and third-party/. MAME retains its original GPL/BSD notices in its separate archive.

This is a curated browser workspace. Its Git history starts with the public source snapshot. Desktop hosts, firmware, credentials and user projects are excluded.
