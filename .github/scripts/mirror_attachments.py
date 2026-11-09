#!/usr/bin/env python3
"""Copies the files people attach to issues (screenshots, videos, report zips, saves) to the issue-attachments branch,
under attachments/<issue number>/, so they can be read through the repository. Issue text is untrusted: nothing from it
is run, links are only fetched from GitHub's own attachment hosts, and file names are cleaned before saving.

usage: mirror_attachments.py OUT_DIR ISSUE [ISSUE ...]   (GITHUB_TOKEN and GITHUB_REPOSITORY from the environment)
"""
import json
import os
import re
import sys
import urllib.request

API = "https://api.github.com"
# the hosts GitHub serves attachments from; anything else in an issue is left alone
LINK = re.compile(
    r"https://(?:github\.com/user-attachments/(?:assets|files)/[\w./%-]+"
    r"|private-user-images\.githubusercontent\.com/[\w./%-]+(?:\?[\w.=&%-]+)?"
    r"|user-images\.githubusercontent\.com/[\w./%-]+)"
)
MAX_BYTES = 50 * 1024 * 1024


def get(url, token, accept="application/vnd.github.full+json"):
    headers = {"Accept": accept, "User-Agent": "attachments-mirror"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=60) as r:
        return r.read()


def texts(repo, n, token):
    """The issue's and its comments' bodies, as written and as GitHub renders them (the rendered one has signed links)."""
    issue = json.loads(get(f"{API}/repos/{repo}/issues/{n}", token))
    out = [issue.get("body") or "", issue.get("body_html") or ""]
    page = 1
    while True:
        cs = json.loads(get(f"{API}/repos/{repo}/issues/{n}/comments?per_page=100&page={page}", token))
        for c in cs:
            out += [c.get("body") or "", c.get("body_html") or ""]
        if len(cs) < 100:
            return out
        page += 1


def clean(name):
    name = re.sub(r"[^\w.-]", "_", name)[:100].lstrip(".")
    return name or "file"


def main():
    out_dir, issues = sys.argv[1], sys.argv[2:]
    token, repo = os.environ.get("GITHUB_TOKEN", ""), os.environ["GITHUB_REPOSITORY"]
    for n in issues:
        if not n.isdigit():
            continue
        links = set()
        for t in texts(repo, n, token):
            links |= {u.replace("&amp;", "&") for u in LINK.findall(t)}
        # a signed copy and the plain link of the same asset share its id: keep one of each id, the signed one first
        seen = {}
        for u in sorted(links, key=lambda u: "private-user-images" not in u):
            m = re.search(r"([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})|/files/(\d+)/", u)
            key = (m.group(1) or m.group(2)) if m else u
            seen.setdefault(key, u)
        d = os.path.join(out_dir, "attachments", n)
        for key, u in seen.items():
            base = clean(u.split("?")[0].rsplit("/", 1)[-1])
            if os.path.exists(d) and any(key in f or f == base for f in os.listdir(d)):
                continue
            try:
                req = urllib.request.Request(u, headers={"User-Agent": "attachments-mirror"})
                with urllib.request.urlopen(req, timeout=120) as r:
                    data = r.read(MAX_BYTES + 1)
                    ctype = r.headers.get("Content-Type", "")
            except Exception as e:  # a link that has gone, or a host that refuses: skip it
                print(f"#{n}: could not fetch {u.split('?')[0]}: {e}")
                continue
            if len(data) > MAX_BYTES:
                print(f"#{n}: skipped {base} (over 50 MB)")
                continue
            name = base
            if "." not in name.rsplit("-", 1)[-1]:
                ext = {"image/png": ".png", "image/jpeg": ".jpg", "image/gif": ".gif", "video/mp4": ".mp4", "video/quicktime": ".mov",
                       "application/zip": ".zip", "text/plain": ".txt"}.get(ctype.split(";")[0].strip(), "")
                name += ext
            os.makedirs(d, exist_ok=True)
            with open(os.path.join(d, name), "wb") as f:
                f.write(data)
            print(f"#{n}: saved {name} ({len(data)} bytes)")


if __name__ == "__main__":
    main()
