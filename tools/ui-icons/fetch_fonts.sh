#!/usr/bin/env bash
# Downloads the HUD fonts (SIL Open Font Licence 1.1) into unity/Assets/Resources/UI/Fonts and verifies
# each file against the SHA-256 recorded in docs/unity/THIRD_PARTY.md. Static TTF instances come from the
# Google Fonts CSS API (fonts.gstatic.com); the licence texts from the google/fonts repository.
# Then run: python3 tools/ui-icons/build_icons.py   (writes the importer metas)
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
fonts="$root/unity/Assets/Resources/UI/Fonts"
mkdir -p "$fonts"

fetch() { # file url sha256
  local file="$1" url="$2" sha="$3" target="$fonts/$1"
  if [ -f "$target" ] && [ "$(shasum -a 256 "$target" | cut -d' ' -f1)" = "$sha" ]; then
    echo "fetch_fonts: $file ok"
    return 0
  fi
  curl -sSfL -o "$target.tmp" "$url"
  local got
  got="$(shasum -a 256 "$target.tmp" | cut -d' ' -f1)"
  if [ "$got" != "$sha" ]; then
    rm -f "$target.tmp"
    echo "fetch_fonts: $file has SHA-256 $got, expected $sha" >&2
    exit 1
  fi
  mv "$target.tmp" "$target"
  echo "fetch_fonts: $file downloaded"
}

fetch Manrope-Regular.ttf https://fonts.gstatic.com/s/manrope/v20/xn7_YHE41ni1AdIRqAuZuw1Bx9mbZk79FO_F.ttf a13d9b41b0a471ce58f0e46d377fa3cc76615e4632c3b15eb397caadf7a13f0a
fetch Manrope-SemiBold.ttf https://fonts.gstatic.com/s/manrope/v20/xn7_YHE41ni1AdIRqAuZuw1Bx9mbZk4jE-_F.ttf 6bad1a774228464cc88b8b0271555b266f7a3c64a7bedf15e165fdab4f6ac0ce
fetch Manrope-Bold.ttf https://fonts.gstatic.com/s/manrope/v20/xn7_YHE41ni1AdIRqAuZuw1Bx9mbZk4aE-_F.ttf b9584e099e0e7f1e1e914c3037aabb9010c50346a3ce8df5eeeeecc0563044d8
fetch Manrope-ExtraBold.ttf https://fonts.gstatic.com/s/manrope/v20/xn7_YHE41ni1AdIRqAuZuw1Bx9mbZk59E-_F.ttf b8f802f05b322a7345805f5b89f8bb976604cc719a91530918d5ce3c47160950
fetch InstrumentSerif-Italic.ttf https://fonts.gstatic.com/s/instrumentserif/v5/jizHRFtNs2ka5fXjeivQ4LroWlx-6zATiw.ttf fecf4861d5202405b8bd45c683858b6870642a026784fe0dbf88ff55dedee8a0
fetch OFL-Manrope.txt https://raw.githubusercontent.com/google/fonts/main/ofl/manrope/OFL.txt 58172e0c0fac2cda8a37b348164bb55e44b0e69051e557e92b1d3f6910141f7b
fetch OFL-InstrumentSerif.txt https://raw.githubusercontent.com/google/fonts/main/ofl/instrumentserif/OFL.txt 129ed7618959716959f2941fdd5b49e0ad6e6c1d78726761786a00253d865521
