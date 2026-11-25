"""Acceptance checks for site/windows/index.html (SEO page for Windows 10/11)."""
import json
import re
import sys
from html.parser import HTMLParser
from pathlib import Path

PAGE = Path(__file__).resolve().parent.parent / "site" / "windows" / "index.html"
URL = "https://adapt-to-it.github.io/hantek-oscilloscope/windows/"


class Collect(HTMLParser):
    def __init__(self):
        super().__init__()
        self.tags, self.stack, self.text = [], [], {}
        self.ld = []

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        self.tags.append((tag, a))
        self.stack.append((tag, a))
        self.text.setdefault(tag, []).append("")

    def handle_endtag(self, tag):
        if self.stack and self.stack[-1][0] == tag:
            t, a = self.stack.pop()
            if t == "script" and a.get("type") == "application/ld+json":
                self.ld.append(self.text["script"][-1])

    def handle_data(self, data):
        for t, _ in self.stack[-1:]:
            self.text[t][-1] += data


def main():
    errors = []
    html = PAGE.read_text(encoding="utf-8")
    p = Collect()
    p.feed(html)
    title = "".join(p.text.get("title", [""]))
    h1 = p.text.get("h1", [])
    meta = {a.get("name"): a.get("content", "") for t, a in p.tags if t == "meta" and a.get("name")}
    links = [a.get("href", "") for t, a in p.tags if t == "a"]
    canon = [a.get("href") for t, a in p.tags if t == "link" and a.get("rel") == "canonical"]
    low = html.lower()

    for word in ("hantek 6022be", "windows 11", "driver"):
        if word not in title.lower():
            errors.append(f"title lacks '{word}': {title!r}")
    if not 40 <= len(title) <= 70:
        errors.append(f"title length {len(title)} not in 40..70")
    if len(h1) != 1 or "windows" not in h1[0].lower():
        errors.append(f"need exactly one h1 mentioning Windows, got {h1}")
    d = meta.get("description", "")
    if not 120 <= len(d) <= 165 or "windows 11" not in d.lower():
        errors.append(f"description must be 120..165 chars and mention Windows 11: {len(d)}")
    if canon != [URL]:
        errors.append(f"canonical must be {URL}, got {canon}")
    for needed in ("04b4 6022", "04b5 6022", "winusb", "zadig.akeo.ie", "list all devices",
                   "replace driver", "device manager", "x64-setup.exe", "_x64_en-us.msi",
                   "arm64-setup.exe", "windows 10"):
        if needed not in low:
            errors.append(f"page lacks '{needed}'")
    if not any(h.endswith("/releases/latest") for h in links):
        errors.append("no link to /releases/latest")
    if not any(h in ("../", "https://adapt-to-it.github.io/hantek-oscilloscope/") for h in links):
        errors.append("no link back to the home page")
    if "../assets/" not in html:
        errors.append("assets must be referenced as ../assets/")
    types = []
    for block in p.ld:
        try:
            data = json.loads(block)
        except json.JSONDecodeError as e:
            errors.append(f"invalid JSON-LD: {e}")
            continue
        for item in data if isinstance(data, list) else data.get("@graph", [data]):
            types.append(item.get("@type"))
    for t in ("FAQPage",):
        if t not in types:
            errors.append(f"JSON-LD lacks {t}")
    if len(re.findall(r"<h2\b", html)) < 4:
        errors.append("need at least 4 h2 sections")

    for e in errors:
        print("FAIL:", e)
    print("ok" if not errors else f"{len(errors)} failures")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
