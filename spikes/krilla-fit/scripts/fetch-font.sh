#!/usr/bin/env sh
# Downloads Noto Sans Regular (SIL OFL 1.1, see fonts/OFL.txt) into fonts/ and checks its hash.
# The font is not committed (620 KB); the spike reads it from fonts/NotoSans-Regular.ttf.
set -eu
cd "$(dirname "$0")/.."
url="https://github.com/notofonts/notofonts.github.io/raw/main/fonts/NotoSans/hinted/ttf/NotoSans-Regular.ttf"
sha="478c558ea716033cd60c03438f628dfa75694dcf6b5f6d505a2f05fd2b4f3823"
curl -sSfL -o fonts/NotoSans-Regular.ttf "$url"
echo "$sha  fonts/NotoSans-Regular.ttf" | shasum -a 256 -c -
