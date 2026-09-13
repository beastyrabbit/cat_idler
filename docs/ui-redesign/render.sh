#!/bin/zsh
# usage: render.sh <html> <out.png> [WxH]
set -e
SZ=${3:-1920,1080}
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 \
  --window-size=${SZ} --virtual-time-budget=6000 --screenshot="$2" "file://$(cd "$(dirname "$1")" && pwd)/$(basename "$1")" >/dev/null 2>&1
file "$2"
