"""Check every text/background token pair in src/app.css against WCAG AA (4.5:1),
in both themes.  Usage: python3 scripts/check-contrast.py"""
import re
css = open(__import__('pathlib').Path(__file__).resolve().parent.parent / 'src' / 'app.css').read()
def block(sel_regex):
    m = re.search(sel_regex + r'\s*\{(.*?)\n\s*\}', css, re.S)
    return dict(re.findall(r'--([\w-]+):\s*(#[0-9a-fA-F]{6})', m.group(1)))
light = block(r'(?m)^:root')
dark = {**light, **block(r':root\[data-theme="dark"\]')}
def lum(h):
    c = [int(h[i:i+2], 16) / 255 for i in (1, 3, 5)]
    c = [x / 12.92 if x <= 0.03928 else ((x + 0.055) / 1.055) ** 2.4 for x in c]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
def ratio(a, b):
    la, lb = sorted([lum(a), lum(b)], reverse=True)
    return (la + 0.05) / (lb + 0.05)
pairs = [('ink','sheet'),('muted','sheet'),('muted','lightbox'),('muted','hover'),('cyan','sheet'),('cyan','lightbox'),
         ('cyan','selected'),('cyan-on','cyan-solid'),('magenta','sheet'),('magenta','magenta-soft'),('red','sheet'),
         ('red','red-soft'),('red-on','red-solid'),('amber','sheet'),('amber','amber-soft'),('ink','raised')]
bad = 0
for name, t in (('light', light), ('dark', dark)):
    for fg, bg in pairs:
        r = ratio(t[fg], t[bg]); ok = r >= 4.5
        bad += not ok
        print(f"{name:5} {fg:>9} on {bg:<12} {r:5.2f}{'' if ok else '  FAIL'}")
print('failures:', bad)
raise SystemExit(1 if bad else 0)
