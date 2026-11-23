"""Validate extracted collection faces with fontTools, an independent parser.

    cargo run -p font-core --example probe --release -- /usr/share/fonts /tmp/extract
    python3 scripts/validate_extraction.py /tmp/extract /usr/share/fonts

For every extracted face: strict per-table checksums, a correct
head.checkSumAdjustment, and identical glyph order, cmap, name records and
table set compared with face N of the source collection.
"""
import glob, os, re, sys
from fontTools.ttLib import TTFont

extract_dir, source_root = sys.argv[1], sys.argv[2]
ok = bad = 0
for f in sorted(glob.glob(os.path.join(extract_dir, '*'))):
    stem, idx = re.match(r'(.*)__(\d+)\.\w+$', os.path.basename(f)).groups()
    idx = int(idx)
    src = next(iter(glob.glob(f'{source_root}/**/{glob.escape(stem)}.[ot]t[cf]', recursive=True)))
    try:
        ext = TTFont(f, checkChecksums=2)  # raises on any table checksum mismatch
        data = open(f, 'rb').read()
        n = int.from_bytes(data[4:6], 'big')
        head = next(int.from_bytes(data[12 + i*16 + 8:12 + i*16 + 12], 'big')
                    for i in range(n) if data[12 + i*16:16 + i*16] == b'head')
        adj = int.from_bytes(data[head + 8:head + 12], 'big')
        z = bytearray(data); z[head + 8:head + 12] = b'\0\0\0\0'
        z += b'\0' * ((4 - len(z) % 4) % 4)
        total = sum(int.from_bytes(z[i:i+4], 'big') for i in range(0, len(z), 4)) & 0xFFFFFFFF
        assert adj == (0xB1B0AFBA - total) & 0xFFFFFFFF, 'checkSumAdjustment'

        orig = TTFont(src, fontNumber=idx)
        assert ext.getGlyphOrder() == orig.getGlyphOrder(), 'glyph order'
        assert ext.getBestCmap() == orig.getBestCmap(), 'cmap'
        for nid in (1, 2, 4, 6, 16, 17):
            assert ext['name'].getDebugName(nid) == orig['name'].getDebugName(nid), f'name {nid}'
        assert set(ext.keys()) == set(orig.keys()) - {'DSIG'}, 'table set'
        ok += 1
    except Exception as e:
        bad += 1
        print('FAIL', os.path.basename(f), type(e).__name__, e)
print(f'{ok} faces valid, {bad} failed')
sys.exit(1 if bad else 0)
