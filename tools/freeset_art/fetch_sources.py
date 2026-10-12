#!/usr/bin/env python3
"""Downloads the CC0 recordings sounds.py uses into a folder (not committed): Kenney's Impact Sounds, RPG Audio and
Music Jingles (kenney.nl, CC0 1.0; taken from a public mirror that keeps their file names) and the uisfx interface sounds
(github.com/romainsimon/uisfx, CC0 1.0, as published on npm in @wubbleai/community-sfx).

usage: fetch_sources.py DIR"""
import io
import json
import os
import ssl
import sys
import tarfile
import urllib.parse
import urllib.request

ctx = ssl.create_default_context(cafile=os.environ.get("SSL_CERT_FILE") or None)
RAW = "https://raw.githubusercontent.com/openpatch/scratch-for-java/main/src/main/resources/"
CATALOG = "https://raw.githubusercontent.com/openpatch/scratch-for-java/main/catalogs/assets.json"
UISFX = "https://registry.npmjs.org/@wubbleai/community-sfx/-/community-sfx-0.1.0.tgz"


def get(url):
    with urllib.request.urlopen(url, context=ctx, timeout=120) as r:
        return r.read()


def main():
    out = sys.argv[1]
    cat = json.loads(get(CATALOG))
    n = 0
    for snd in cat["sounds"]:
        dst = os.path.join(out, "kenney", snd["path"])
        if os.path.exists(dst):
            continue
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        open(dst, "wb").write(get(RAW + urllib.parse.quote(snd["path"])))
        n += 1
    print(n, "Kenney sounds")
    tar = tarfile.open(fileobj=io.BytesIO(get(UISFX)), mode="r:gz")
    m = 0
    for member in tar.getmembers():
        parts = member.name.split("/")
        # package/catalog/sounds/<flavour>/<name>.ogg
        if len(parts) == 5 and parts[1:3] == ["catalog", "sounds"] and parts[4].endswith(".ogg") and member.isfile():
            dst = os.path.join(out, "uisfx", parts[3], parts[4])
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            open(dst, "wb").write(tar.extractfile(member).read())
            m += 1
    print(m, "uisfx sounds")


if __name__ == "__main__":
    main()
