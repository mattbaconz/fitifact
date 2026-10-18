# FFmpeg in the Fitifact desktop app

The Windows desktop app ships two FFmpeg programs, `ffmpeg.exe` and
`ffprobe.exe`, next to `fitifact-desktop.exe`. Fitifact runs them as separate
processes to inspect and change video and audio. Fitifact does not link FFmpeg,
and Fitifact's Apache-2.0 license does not cover them.

## The shipped build

- FFmpeg 9.0.2, the 64-bit Windows "essentials" build by Gyan Doshi
  (https://www.gyan.dev/ffmpeg/builds/).
- Archive:
  https://github.com/GyanD/codexffmpeg/releases/download/9.0.2/ffmpeg-9.0.2-essentials_build.zip
- Archive SHA-256:
  `60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba`
- `scripts/fetch-desktop-ffmpeg.ps1` downloads that archive, checks its hash,
  and copies the two programs unchanged. Fitifact does not sign them.

## License

The build is configured with `--enable-gpl --enable-version3`, so these
programs are licensed under the GNU General Public License, version 3. The
installer puts the full license text in `FFMPEG-LICENSE.txt` beside this file.
`FFMPEG-BUILD.txt`, the build's own README, lists its configuration and the
version of every external library it contains (for example x264, x265, libvpx,
and LAME).

## Source code

- FFmpeg source for this build:
  https://github.com/FFmpeg/FFmpeg/commit/946fcce07b, also released as
  https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz.
- External libraries: `FFMPEG-BUILD.txt` names each one with its version; each
  project publishes its own source.
- Build provider: https://www.gyan.dev/ffmpeg/builds/ and
  https://github.com/GyanD/codexffmpeg.

If one of these links stops working, open an issue at
https://github.com/mattbaconz/fitifact/issues to get the source.

The Fitifact command-line tool does not ship FFmpeg. It uses the FFmpeg on your
PATH, under that build's license.
