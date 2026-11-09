// SimGolf in the browser: reading the player's files before the game starts (no page or game state here, so the same file
// runs under node for web/test.js). A .zip is read entry by entry from the picked file (iPad Safari cannot hold a whole
// game zip and its unpacked files at once) and unpacked here: stored, deflate, Deflate64, BZip2, LZMA and PPMd entries,
// the methods Windows' zipper, 7-Zip, The Unarchiver and Keka write (iPad's Files app and the browser's own
// DecompressionStream only know deflate). Then where each file goes in the game folder (sgPlace) and what the player is
// told about it (sgFilesSummary). opensgolf.js keeps the files and starts the game.
"use strict";

// The page's version, stamped by web/build.sh (the start page shows it, so a player can tell an old copy kept by the
// browser from the current one).
const SG_VERSION = "%SG_VERSION%";
function sgVersion() {
    return SG_VERSION.startsWith("%") ? "dev" : SG_VERSION;
}

// --- phones and tablets held upright (the port's own; web/opensgolf.js sgFitCanvas) ------------------------------------------
// SimGolf is a landscape game: its screens are 800 x 600. A phone or tablet (a touch device: a coarse pointer, or an iPad
// that calls itself a Mac) held upright (taller than wide) is asked to turn sideways, until the player picks "Play anyway"
// for the session. A desktop window, however tall, is never asked.
function sgPortraitAsk(touch, w, h, dismissed) {
    return !!touch && h > w && !dismissed;
}

// Where the game's canvas goes in the visible area (w x h at left, top): all of it, except on a touch device held upright,
// where the game plays in a 4:3 band across the middle (the 800 x 600 screens fill its width; the rest is black), so the
// whole game shows at the largest size the width allows, neither cut off nor stretched.
function sgCanvasRect(touch, w, h, left, top) {
    if (!touch || h <= w) return { left: Math.round(left), top: Math.round(top), width: Math.round(w), height: Math.round(h) };
    const bh = Math.min(h, w * 3 / 4);
    return { left: Math.round(left), top: Math.round(top + (h - bh) / 2), width: Math.round(w), height: Math.round(bh) };
}

// The edges of the canvas a phone hides (its camera cut-out, rounded corners and home bar), once the page draws under them
// (index.html viewport-fit=cover): the window's CSS safe-area insets [left, top, right, bottom] where they fall on the canvas
// (rect, in a w x h window), as fractions of the canvas's width and height, for the game (ui.rs safe_extent). An upright
// phone's 4:3 band in the middle reaches neither its top nor its bottom inset.
function sgSafeInsets(insets, rect, w, h) {
    const [l, t, r, b] = insets;
    if (!(rect.width > 0 && rect.height > 0)) return [0, 0, 0, 0];
    const over = (inset, gap) => Math.max(0, inset - Math.max(0, gap));
    return [
        over(l, rect.left) / rect.width,
        over(t, rect.top) / rect.height,
        over(r, w - rect.left - rect.width) / rect.width,
        over(b, h - rect.top - rect.height) / rect.height,
    ];
}

// --- the game's sound asleep in the background (the port's own; web/opensgolf.js sgSoundSleep) ----------------------------
// What the page does with its sound context, in `state` ("running", "suspended", Safari's "interrupted", "closed", or none
// made yet): "suspend" while the game has its sound asleep (the Pause in the background extra, with the window in the
// background), "resume" for a context that is not running while the game's sound is awake and the page is shown, else
// "". A context left suspended plays nothing, so a phone has no sound to keep the page running for.
function sgSoundAction(state, asleep, hidden) {
    if (!state || state === "closed") return "";
    if (asleep) return state === "running" ? "suspend" : "";
    return state !== "running" && !hidden ? "resume" : "";
}

function sgKey(p) {
    return p.replace(/\\/g, "/").replace(/\/+/g, "/").replace(/\/$/, "").replace(/^\.\//, "").toLowerCase();
}

function sgPlural(n, word) {
    return n + " " + word + (n === 1 ? "" : "s");
}

// A sound file, by its name (the game plays WAV, MP3, MP2 and OGG).
function sgIsSound(path) {
    return /\.(wav|mp3|mp2|ogg)$/i.test(path);
}

// The coding of a sound file the game cannot play, by its headers (the same check as sg_core::assets::probe_sound, for the
// summary: a copy whose sounds were shrunk to such a coding played the free sounds with nothing said), or null when it
// plays: WAV in PCM, float, A-law, mu-law, MS and IMA ADPCM, GSM 6.10, MPEG layers 1-3; AIFF; bare MP3/MP2; OGG Vorbis.
const SG_CODECS = { 0x10: "OKI ADPCM", 0x14: "G.721/G.726 ADPCM", 0x22: "DSP Group TrueSpeech", 0x32: "MSN Audio", 0x40: "G.721/G.726 ADPCM",
    0x42: "G.723.1", 0x64: "G.721/G.726 ADPCM", 0x92: "Dolby AC-3", 0x130: "Sipro ACELP.net", 0x160: "Windows Media Audio",
    0x161: "Windows Media Audio", 0x162: "Windows Media Audio", 0x163: "Windows Media Audio", 0x401: "Intel Music Coder", 0x1600: "AAC",
    0x1601: "AAC", 0xa106: "AAC", 0xf1ac: "FLAC" };
function sgSoundCoding(b) {
    if (!(b instanceof Uint8Array)) b = new Uint8Array(b);
    const s = (o, n) => String.fromCharCode(...b.subarray(o, o + n));
    if (b.length < 4) return "not a WAV, AIFF, MP3 or OGG file";
    if (s(0, 4) === "OggS") {
        const head = s(0, Math.min(b.length, 256));
        return head.includes("\x01vorbis") ? null : head.includes("OpusHead") ? "Opus" : head.includes("Speex") ? "Speex"
            : head.includes("FLAC") ? "FLAC" : "an OGG file that is not Vorbis";
    }
    if (s(0, 3) === "ID3" || (b[0] === 0xff && (b[1] & 0xe0) === 0xe0)) return null;
    if (s(0, 4) === "fLaC") return "FLAC";
    if (b.length >= 12 && s(0, 4) === "FORM" && s(8, 4) === "AIFF") return null;
    if (b.length >= 12 && s(0, 4) === "FORM" && s(8, 4) === "AIFC") {
        // AIFF-C: its COMM chunk's compression
        const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
        for (let pos = 12; pos + 8 <= b.length;) {
            const sz = dv.getUint32(pos + 4);
            if (s(pos, 4) === "COMM" && pos + 30 <= b.length) {
                const c = s(pos + 26, 4);
                return ["NONE", "twos", "sowt", "fl32", "FL32", "fl64", "FL64", "ulaw", "ULAW", "alaw", "ALAW"].includes(c) ? null
                    : "AIFF-C coding '" + c + "'";
            }
            pos += 8 + sz + (sz & 1);
        }
        return "AIFF with no COMM chunk";
    }
    if (b.length >= 12 && s(4, 4) === "ftyp") return "AAC/M4A";
    if (!["RIFF", "RIFX", "RF64"].includes(s(0, 4)) || b.length < 12 || s(8, 4) !== "WAVE") return "not a WAV, AIFF, MP3 or OGG file";
    const big = s(0, 4) === "RIFX";
    const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
    const u16 = o => dv.getUint16(o, !big), u32 = o => dv.getUint32(o, !big);
    for (let pos = 12; pos + 8 <= b.length;) {
        const sz = u32(pos + 4);
        if (s(pos, 4) === "fmt " && sz >= 16 && pos + 24 <= b.length) {
            let tag = u16(pos + 8);
            const bits = u16(pos + 22);
            if (tag === 0xfffe && pos + 34 <= b.length) tag = u16(pos + 32);
            const names = { 1: "PCM", 2: "Microsoft ADPCM", 3: "float PCM", 6: "A-law", 7: "mu-law", 0x11: "IMA ADPCM" };
            const ok = { 1: [8, 16, 24, 32], 3: [32, 64], 6: [8], 7: [8], 2: [4], 0x11: [4] };
            if (big && ![1, 3].includes(tag)) return (names[tag] || SG_CODECS[tag] || "WAV format 0x" + tag.toString(16).padStart(4, "0")) + " in a big-endian (RIFX) WAV";
            if (ok[tag]) return ok[tag].includes(bits) ? null : bits + "-bit " + names[tag];
            if ([0x31, 0x50, 0x55].includes(tag)) return null;
            return SG_CODECS[tag] || "WAV format 0x" + tag.toString(16).padStart(4, "0");
        }
        pos += 8 + sz + (sz & 1);
    }
    return "no fmt chunk";
}

// Files the game does not need. Videos (.bik) and Windows binaries are skipped.
function sgSkip(low) {
    // a zip made on a Mac carries the files' resource forks in __MACOSX/ and as "._" files: not game files
    if (low.startsWith("__macosx/") || low.includes("/__macosx/") || low.startsWith("._") || low.includes("/._")) return true;
    return low.endsWith(".bik") || low.endsWith(".exe") || low.endsWith(".dll") || low.endsWith(".lib") || low.endsWith("/");
}

// The game folder in a set of paths: the folder holding Flics. Of several (a zip with a second copy, or a stray Flics
// folder), the one with the most of the game's other folders beside it, then the shallowest. Folders differing only in
// case are one (a zip made on Windows may hold "SimGolf/Flics/..." and "SIMGOLF/Sounds/..."); the spelling returned is the
// first seen.
function sgRootOf(paths) {
    const roots = new Map();
    for (const p of paths) {
        const parts = p.split("/");
        const i = parts.slice(0, -1).findIndex(x => x.toLowerCase() === "flics");
        if (i >= 0) {
            const r = parts.slice(0, i).join("/");
            if (!roots.has(r.toLowerCase())) roots.set(r.toLowerCase(), { name: r, kids: new Set() });
        }
    }
    if (roots.size === 0) return null;
    for (const p of paths) {
        const parts = p.split("/");
        for (let i = 0; i < parts.length - 1; i++) {
            const r = roots.get(parts.slice(0, i).join("/").toLowerCase());
            if (r) r.kids.add(parts[i].toLowerCase());
        }
    }
    const score = r => ["interface", "data", "sounds", "simsfx"].filter(d => r.kids.has(d)).length;
    return Array.from(roots.values()).sort((a, b) => score(b) - score(a) || a.name.split("/").length - b.name.split("/").length)[0].name;
}

// A sound file's place in the game folder when it sits in a sound folder somewhere else (a copy repacked with Sounds or
// SimsFX beside the game folder, a second copy of the game, a folder named "SimGolf Sounds"): "Sounds/..." or
// "SimsFX/...", from the deepest such folder; null for any other file.
function sgStraySound(path) {
    if (!sgIsSound(path)) return null;
    const parts = path.split("/");
    for (let i = parts.length - 2; i >= 0; i--) {
        const d = parts[i].toLowerCase().replace(/[ _\-.]/g, "");
        const name = d === "sound" || d.endsWith("sounds") ? "Sounds" : d.includes("simsfx") ? "SimsFX" : null;
        if (name) return name + "/" + parts.slice(i + 1).join("/");
    }
    return null;
}

// Where each of `paths` goes under the game folder: [index, path under the game folder]. The files under the game folder
// (sgRootOf, any case) keep their place; a sound file anywhere else in a sound folder (sgStraySound), or in a sound
// folder under the game folder that the game does not look in, also goes where the game looks, unless a file is there
// already. Returns null when there is no game folder.
function sgPlace(paths) {
    const root = sgRootOf(paths);
    if (root === null) return null;
    const low = root.toLowerCase();
    const out = [], have = new Set(), strays = [];
    paths.forEach((p, i) => {
        const under = low === "" || p.toLowerCase().startsWith(low + "/");
        const rel = under ? p.substring(root.length).replace(/^\//, "") : null;
        if (rel !== null) {
            out.push([i, rel]);
            have.add(sgKey(rel));
        }
        const top = rel === null ? null : sgKey(rel).split("/")[0];
        if (top !== "sounds" && top !== "simsfx") {
            const stray = sgStraySound(rel === null ? p : rel);
            if (stray !== null) strays.push([i, stray]);
        }
    });
    for (const [i, rel] of strays) {
        if (have.has(sgKey(rel))) continue;
        have.add(sgKey(rel));
        out.push([i, rel]);
    }
    return out;
}

// --- reading a .zip ----------------------------------------------------------------------------------------------------------
// The compression methods, by their zip number, for the player's message.
const SG_ZIP_METHODS = { 0: "stored", 8: "Deflate", 9: "Deflate64", 12: "BZip2", 14: "LZMA", 93: "Zstandard", 95: "XZ", 96: "JPEG",
    97: "WavPack", 98: "PPMd", 99: "AES encryption" };
function sgMethodName(m) {
    return SG_ZIP_METHODS[m] || "compression method " + m;
}
// The largest entry unpacked (the game's biggest file is a few MB; an iPad tab has a few hundred MB in all).
const SG_ZIP_MAX = 512 * 1024 * 1024;

// The checksum of zip entries.
const sgCrcTable = (() => {
    const t = new Uint32Array(256);
    for (let n = 0; n < 256; n++) {
        let c = n;
        for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
        t[n] = c >>> 0;
    }
    return t;
})();
function sgCrc32(b) {
    let c = 0xffffffff;
    for (let i = 0; i < b.length; i++) c = sgCrcTable[(c ^ b[i]) & 0xff] ^ (c >>> 8);
    return (c ^ 0xffffffff) >>> 0;
}

// Deflate64 (zip method 9), which Windows' built-in zipper uses for big files: a zip of SimGolf made there can hold every
// sound so, and neither the browser (DecompressionStream) nor iPad's Files app unpacks it, so the game played the free
// sounds with the original's art (issue #10). Deflate with a 64 KiB window, length code 285 taking 16 extra bits over a
// base of 3, and distance codes 30 and 31. The same decoder as crates/sg-core/src/inflate64.rs (after zlib's puff.c).
// `size` is the entry's size from the zip's directory. sgInflateInto also decodes plain deflate (d64 false: zip method 8
// when the browser's DecompressionStream is missing or fails, and the blocks of the CD's installer cabinets) into a part
// of a bigger buffer.
const SG_LBASE = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 3];
const SG_LEXT = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 16];
const SG_DBASE = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097,
    6145, 8193, 12289, 16385, 24577, 32769, 49153];
const SG_DEXT = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 14, 14];
// A canonical Huffman code: counts of each length, the symbols in code order, and (for speed) a table of the codes up to
// 9 bits by their next 9 bits as read (symbol + length << 16; 0 for a longer code).
function sgHuff(lengths) {
    const count = new Uint16Array(16), offs = new Uint16Array(16), symbol = new Uint16Array(lengths.length);
    for (const l of lengths) count[l]++;
    count[0] = 0;
    for (let l = 1; l < 15; l++) offs[l + 1] = offs[l] + count[l];
    for (let s = 0; s < lengths.length; s++) if (lengths[s]) symbol[offs[lengths[s]]++] = s;
    const fast = new Uint32Array(512);
    let code = 0, k = 0;
    for (let l = 1; l <= 9; l++) {
        for (let j = 0; j < count[l]; j++, k++, code++) {
            let rev = 0;
            for (let b = 0; b < l; b++) rev |= ((code >> b) & 1) << (l - 1 - b);
            for (let x = rev; x < 512; x += 1 << l) fast[x] = symbol[k] | (l << 16);
        }
        code <<= 1;
    }
    return { count, symbol, fast };
}
function sgInflate(src, size, d64) {
    const out = new Uint8Array(size);
    if (sgInflateInto(src, out, 0, d64) !== size) throw new Error((d64 ? "Deflate64" : "deflate") + ": less data than the zip says");
    return out;
}
function sgInflate64(src, size) {
    return sgInflate(src, size, true);
}
// Decodes `src` into `out` from `start`; returns where its output ends.
function sgInflateInto(src, out, start, d64) {
    const size = out.length;
    let o = start, pos = 0, buf = 0, n = 0;
    const fail = why => { throw new Error((d64 ? "Deflate64: " : "deflate: ") + why); };
    const bits = k => {
        while (n < k) {
            if (pos >= src.length) fail("the data ends early");
            buf |= src[pos++] << n;
            n += 8;
        }
        const v = buf & ((1 << k) - 1);
        buf >>>= k;
        n -= k;
        return v;
    };
    const decode = h => {
        while (n < 9 && pos < src.length) {
            buf |= src[pos++] << n;
            n += 8;
        }
        const f = h.fast[buf & 511];
        if (f !== 0 && (f >>> 16) <= n) {
            buf >>>= f >>> 16;
            n -= f >>> 16;
            return f & 0xffff;
        }
        let code = 0, first = 0, index = 0;
        for (let len = 1; len < 16; len++) {
            if (n === 0) {
                if (pos >= src.length) fail("the data ends early");
                buf = src[pos++];
                n = 8;
            }
            code |= buf & 1;
            buf >>>= 1;
            n--;
            const count = h.count[len];
            if (code - count < first) return h.symbol[index + code - first];
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        fail("a bad code");
    };
    const fixedL = new Uint8Array(288).fill(8, 0, 144).fill(9, 144, 256).fill(7, 256, 280).fill(8, 280);
    const fixed = [sgHuff(fixedL), sgHuff(new Uint8Array(32).fill(5))];
    const ORDER = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
    let last;
    do {
        last = bits(1);
        const type = bits(2);
        let lh, dh;
        if (type === 0) {
            buf = 0;
            n = 0;
            if (pos + 4 > src.length) fail("the data ends early");
            const len = src[pos] | (src[pos + 1] << 8);
            if ((len ^ 0xffff) !== (src[pos + 2] | (src[pos + 3] << 8))) fail("a bad stored block");
            if (pos + 4 + len > src.length || o + len > size) fail("a stored block too long");
            out.set(src.subarray(pos + 4, pos + 4 + len), o);
            o += len;
            pos += 4 + len;
            continue;
        } else if (type === 1) {
            [lh, dh] = fixed;
        } else if (type === 2) {
            const nlen = bits(5) + 257, ndist = bits(5) + 1, ncode = bits(4) + 4;
            const cl = new Uint8Array(19);
            for (let i = 0; i < ncode; i++) cl[ORDER[i]] = bits(3);
            const ch = sgHuff(cl);
            const lengths = new Uint8Array(nlen + ndist);
            for (let i = 0; i < nlen + ndist;) {
                const sym = decode(ch);
                let val = 0, rep = 1;
                if (sym < 16) val = sym;
                else if (sym === 16) {
                    if (i === 0) fail("a repeat with nothing before it");
                    val = lengths[i - 1];
                    rep = 3 + bits(2);
                } else rep = sym === 17 ? 3 + bits(3) : 11 + bits(7);
                if (i + rep > nlen + ndist) fail("too many code lengths");
                lengths.fill(val, i, i + rep);
                i += rep;
            }
            lh = sgHuff(lengths.subarray(0, nlen));
            dh = sgHuff(lengths.subarray(nlen));
        } else fail("a bad block type");
        for (;;) {
            const sym = decode(lh);
            if (sym < 256) {
                if (o >= size) fail("more data than the zip says");
                out[o++] = sym;
                continue;
            }
            if (sym === 256) break;
            const s = sym - 257;
            if (s >= 29) fail("a bad length code");
            const len = !d64 && s === 28 ? 258 : SG_LBASE[s] + bits(SG_LEXT[s]);
            const ds = decode(dh);
            if (!d64 && ds >= 30) fail("a bad distance code");
            const dist = SG_DBASE[ds] + bits(SG_DEXT[ds]);
            if (dist > o - start) fail("a distance too far back");
            if (o + len > size) fail("more data than the zip says");
            if (dist >= len) out.copyWithin(o, o - dist, o - dist + len);
            else for (let k = 0; k < len; k++) out[o + k] = out[o - dist + k];
            o += len;
        }
    } while (!last);
    return o;
}

// BZip2 (zip method 12; 7-Zip's and The Unarchiver's "BZip2" zips). The port's own decoder after the format bzip2 1.0
// writes: "BZh" and the block size, then blocks of up to 900 000 bytes, each the Burrows-Wheeler transform of the run
// length coded data, move-to-front coded, its runs of the front symbol as RUNA/RUNB digits and Huffman coded with up to
// six tables switched every 50 symbols. Streams written one after another (a parallel bzip2's) are read on. The same
// decoder as crates/sg-core/src/bunzip2.rs.
function sgBunzip2(src, size) {
    const out = new Uint8Array(size);
    let o = 0, pos = 0, buf = 0, n = 0;
    const fail = why => { throw new Error("BZip2: " + why); };
    // up to 24 bits, the first bit read the highest
    const bits = k => {
        while (n < k) {
            if (pos >= src.length) fail("the data ends early");
            buf = ((buf << 8) | src[pos++]) >>> 0;
            n += 8;
        }
        n -= k;
        return (buf >>> n) & ((1 << k) - 1);
    };
    // a Huffman table: codes up to 10 bits looked up by their next 10 bits (symbol + length << 16), longer ones counted
    const table = lengths => {
        const count = new Uint16Array(21), offs = new Uint16Array(22), symbol = new Uint16Array(lengths.length);
        for (const l of lengths) count[l]++;
        for (let l = 1; l <= 20; l++) offs[l + 1] = offs[l] + count[l];
        for (let s = 0; s < lengths.length; s++) symbol[offs[lengths[s]]++] = s;
        const fast = new Uint32Array(1024);
        let code = 0, k = 0;
        for (let l = 1; l <= 10; l++) {
            for (let j = 0; j < count[l]; j++, k++, code++) {
                const from = code << (10 - l);
                fast.fill(symbol[k] | (l << 16), from, from + (1 << (10 - l)));
            }
            code <<= 1;
        }
        return { count, symbol, fast };
    };
    const decode = h => {
        if (n < 10 && pos + 1 < src.length) {
            buf = ((buf << 16) | (src[pos] << 8) | src[pos + 1]) >>> 0;
            pos += 2;
            n += 16;
        }
        if (n >= 10) {
            const f = h.fast[(buf >>> (n - 10)) & 1023];
            if (f !== 0) {
                n -= f >>> 16;
                return f & 0xffff;
            }
        }
        let code = 0, first = 0, index = 0;
        for (let len = 1; len <= 20; len++) {
            code |= bits(1);
            const count = h.count[len];
            if (code - count < first) return h.symbol[index + code - first];
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        fail("a bad code");
    };
    // (the bits buffered: n is at most 31 here, as no read takes more than 24 at once)
    while (o < size) {
        if (bits(8) !== 0x42 || bits(8) !== 0x5a || bits(8) !== 0x68) fail("not a bzip2 stream");
        const level = bits(8) - 0x30;
        if (level < 1 || level > 9) fail("a bad block size");
        const max = level * 100000;
        const tt = new Uint32Array(max);
        for (;;) {
            const m1 = bits(24), m2 = bits(24);
            if (m1 === 0x177245 && m2 === 0x385090) {
                bits(16);
                bits(16); // the stream's checksum (the zip's own CRC is checked)
                n -= n % 8;
                break;
            }
            if (m1 !== 0x314159 || m2 !== 0x265359) fail("a bad block header");
            bits(16);
            bits(16); // the block's checksum
            if (bits(1)) fail("a randomised block (bzip2 0.9.0's), which is not supported");
            const origPtr = bits(24);
            const used = bits(16), seq = [];
            for (let i = 0; i < 16; i++) {
                if (!(used & (0x8000 >> i))) continue;
                const b = bits(16);
                for (let j = 0; j < 16; j++) if (b & (0x8000 >> j)) seq.push(i * 16 + j);
            }
            if (seq.length === 0) fail("a block with no symbols");
            const alpha = seq.length + 2;
            const nGroups = bits(3), nSel = bits(15);
            if (nGroups < 2 || nGroups > 6 || nSel < 1) fail("bad tables");
            const mtfG = [0, 1, 2, 3, 4, 5].slice(0, nGroups), sel = new Uint8Array(nSel);
            for (let i = 0; i < nSel; i++) {
                let j = 0;
                while (bits(1)) if (++j >= nGroups) fail("a bad table selector");
                const v = mtfG[j];
                mtfG.splice(j, 1);
                mtfG.unshift(v);
                sel[i] = v;
            }
            const groups = [];
            for (let g = 0; g < nGroups; g++) {
                const len = new Uint8Array(alpha);
                let c = bits(5);
                for (let s = 0; s < alpha; s++) {
                    for (;;) {
                        if (c < 1 || c > 20) fail("a bad code length");
                        if (!bits(1)) break;
                        c += bits(1) ? -1 : 1;
                    }
                    len[s] = c;
                }
                groups.push(table(len));
            }
            // the symbols: runs of the front byte (RUNA, RUNB as bijective base-2 digits), else a move to the front
            const mtf = new Uint8Array(256);
            for (let i = 0; i < 256; i++) mtf[i] = i;
            const freq = new Uint32Array(256);
            let count = 0, gi = -1, left = 0, h = null, run = 0, runBit = 1;
            for (;;) {
                if (left === 0) {
                    if (++gi >= nSel) fail("more symbols than tables");
                    h = groups[sel[gi]];
                    left = 50;
                }
                left--;
                const sym = decode(h);
                if (sym <= 1) {
                    run += runBit << sym;
                    runBit <<= 1;
                    if (run > max) fail("a block too long");
                    continue;
                }
                if (run > 0) {
                    const b = seq[mtf[0]];
                    if (count + run > max) fail("a block too long");
                    tt.fill(b, count, count + run);
                    freq[b] += run;
                    count += run;
                    run = 0;
                    runBit = 1;
                }
                if (sym === alpha - 1) break;
                if (count >= max) fail("a block too long");
                const k = sym - 1, v = mtf[k];
                mtf.copyWithin(1, 0, k);
                mtf[0] = v;
                const b = seq[v];
                freq[b]++;
                tt[count++] = b;
            }
            if (origPtr >= count) fail("a bad start of block");
            // the inverse transform: each byte's next one in the low byte's sorted place (bzip2's own tt array)
            const cf = new Uint32Array(256);
            for (let i = 1; i < 256; i++) cf[i] = cf[i - 1] + freq[i - 1];
            for (let i = 0; i < count; i++) tt[cf[tt[i] & 0xff]++] |= i << 8;
            let t = tt[origPtr] >>> 8, last = -1, same = 0;
            for (let i = 0; i < count; i++) {
                const e = tt[t];
                const b = e & 0xff;
                t = e >>> 8;
                if (same === 4) {
                    // four of a byte, then how many more
                    if (o + b > size) fail("more data than the zip says");
                    out.fill(last, o, o + b);
                    o += b;
                    same = 0;
                    last = -1;
                    continue;
                }
                if (b === last) same++;
                else {
                    last = b;
                    same = 1;
                }
                if (o >= size) fail("more data than the zip says");
                out[o++] = b;
            }
        }
        if (o < size && pos >= src.length) break;
    }
    if (o !== size) fail("less data than the zip says");
    return out;
}

// LZMA (zip method 14; 7-Zip's "LZMA" zips): after the zip's own 4 bytes (the LZMA SDK's version, the size of the
// properties) the 5 bytes of properties (lc, lp, pb; the dictionary size, not needed here as the whole entry is decoded
// into one buffer) and the range coded stream. The port's own decoder after the format of the LZMA SDK's LzmaSpec; the
// zip directory's size says where it ends (an end marker, when there is one, is not read). The same decoder as
// crates/sg-core/src/unlzma.rs.
function sgUnlzma(src, size) {
    const fail = why => { throw new Error("LZMA: " + why); };
    if (src.length < 9) fail("the data ends early");
    const psize = src[2] | (src[3] << 8);
    if (psize < 5 || 4 + psize > src.length) fail("bad properties");
    let d = src[4];
    if (d >= 225) fail("bad properties");
    const lc = d % 9;
    d = (d / 9) | 0;
    const lp = d % 5, pb = (d / 5) | 0;
    const out = new Uint8Array(size);
    let pos = 4 + psize;
    if (src[pos] !== 0) fail("a bad start of the stream");
    let range = 0xffffffff, code = 0;
    for (let i = 1; i <= 4; i++) code = ((code << 8) | src[pos + i]) >>> 0;
    pos += 5;
    if (code === range) fail("a bad start of the stream");
    // the probabilities, all in one array
    const IS_MATCH = 0, IS_REP = 192, IS_REP_G0 = 204, IS_REP_G1 = 216, IS_REP_G2 = 228, IS_REP0_LONG = 240, POS_SLOT = 432,
        SPEC_POS = 688, ALIGN = 803, LEN = 819, REP_LEN = 1333, LIT = 1847;
    const p = new Uint16Array(LIT + (0x300 << (lc + lp))).fill(1024);
    const bit = i => {
        const v = p[i], bound = (range >>> 11) * v;
        let b;
        if (code < bound) {
            range = bound;
            p[i] = v + ((2048 - v) >> 5);
            b = 0;
        } else {
            range -= bound;
            code -= bound;
            p[i] = v - (v >> 5);
            b = 1;
        }
        if (range < 0x1000000) {
            range = (range << 8) >>> 0;
            code = ((code << 8) | (pos < src.length ? src[pos] : 0)) >>> 0;
            pos++;
        }
        return b;
    };
    const direct = k => {
        let r = 0;
        for (let i = 0; i < k; i++) {
            range >>>= 1;
            let b = 0;
            if (code >= range) {
                code -= range;
                b = 1;
            }
            r = r * 2 + b;
            if (range < 0x1000000) {
                range = (range << 8) >>> 0;
                code = ((code << 8) | (pos < src.length ? src[pos] : 0)) >>> 0;
                pos++;
            }
        }
        return r;
    };
    const tree = (base, k) => {
        let m = 1;
        for (let i = 0; i < k; i++) m = (m << 1) + bit(base + m);
        return m - (1 << k);
    };
    const reverse = (base, k) => {
        let m = 1, r = 0;
        for (let i = 0; i < k; i++) {
            const b = bit(base + m);
            m = (m << 1) + b;
            r |= b << i;
        }
        return r;
    };
    const length = (base, ps) => {
        if (!bit(base)) return tree(base + 2 + (ps << 3), 3);
        if (!bit(base + 1)) return 8 + tree(base + 130 + (ps << 3), 3);
        return 16 + tree(base + 258, 8);
    };
    const pbMask = (1 << pb) - 1, lpMask = (1 << lp) - 1;
    let o = 0, state = 0, rep0 = 0, rep1 = 0, rep2 = 0, rep3 = 0;
    while (o < size) {
        const ps = o & pbMask;
        if (!bit(IS_MATCH + (state << 4) + ps)) {
            const prev = o > 0 ? out[o - 1] : 0;
            const base = LIT + 0x300 * (((o & lpMask) << lc) + (prev >> (8 - lc)));
            let sym = 1;
            if (state >= 7) {
                if (rep0 >= o) fail("a distance too far back");
                let mb = out[o - rep0 - 1];
                do {
                    const m = (mb >> 7) & 1;
                    mb <<= 1;
                    const b = bit(base + ((1 + m) << 8) + sym);
                    sym = (sym << 1) | b;
                    if (m !== b) break;
                } while (sym < 0x100);
            }
            while (sym < 0x100) sym = (sym << 1) | bit(base + sym);
            out[o++] = sym & 0xff;
            state = state < 4 ? 0 : state < 10 ? state - 3 : state - 6;
            continue;
        }
        let len;
        if (bit(IS_REP + state)) {
            if (o === 0) fail("a repeat at the start");
            if (!bit(IS_REP_G0 + state)) {
                if (!bit(IS_REP0_LONG + (state << 4) + ps)) {
                    state = state < 7 ? 9 : 11;
                    out[o] = out[o - rep0 - 1];
                    o++;
                    continue;
                }
            } else {
                let dist;
                if (!bit(IS_REP_G1 + state)) dist = rep1;
                else {
                    if (!bit(IS_REP_G2 + state)) dist = rep2;
                    else {
                        dist = rep3;
                        rep3 = rep2;
                    }
                    rep2 = rep1;
                }
                rep1 = rep0;
                rep0 = dist;
            }
            len = length(REP_LEN, ps);
            state = state < 7 ? 8 : 11;
        } else {
            rep3 = rep2;
            rep2 = rep1;
            rep1 = rep0;
            len = length(LEN, ps);
            state = state < 7 ? 7 : 10;
            const slot = tree(POS_SLOT + (Math.min(len, 3) << 6), 6);
            if (slot < 4) rep0 = slot;
            else {
                const k = (slot >> 1) - 1;
                let dist = (2 | (slot & 1)) * 2 ** k;
                if (slot < 14) dist += reverse(SPEC_POS + dist - slot, k);
                else dist += direct(k - 4) * 16 + reverse(ALIGN, 4);
                if (dist === 0xffffffff) break; // the end marker
                rep0 = dist;
            }
        }
        len += 2;
        if (rep0 >= o) fail("a distance too far back");
        if (o + len > size) fail("more data than the zip says");
        for (let k = 0; k < len; k++, o++) out[o] = out[o - rep0 - 1];
    }
    if (o !== size) fail("less data than the zip says");
    return out;
}

// PPMd var. I rev. 1 (zip method 98; 7-Zip's "PPMd" zips): a context model of up to 16 bytes before each byte, coded with
// Subbotin's carryless range coder. The zip's two bytes first: the model's order - 1 (4 bits), its memory in MB - 1 (8
// bits) and what is done when the memory is full (4 bits: start again, or cut the model down; the third, "freeze", is
// not supported, as in 7-Zip). A port of 7-Zip's Ppmd8.c and Ppmd8Dec.c (Igor Pavlov, after Dmitry Shkarin's PPMd var.I;
// both public domain): the model lives in one buffer as theirs does (12-byte units: contexts, the 6-byte symbol states
// two to a unit, free-list nodes), so it decodes the same.
function sgUnppmd(src, size) {
    const fail = why => { throw new Error("PPMd: " + why); };
    if (src.length < 2) fail("the data ends early");
    const head = src[0] | (src[1] << 8);
    const maxOrder = (head & 15) + 1, memSize = (((head >> 4) & 0xff) + 1) * 1048576, restore = head >>> 12;
    if (maxOrder < 2 || restore > 1) fail("unsupported settings (order " + maxOrder + ", method " + restore + ")");
    const UNIT = 12, MAX_FREQ = 124, NI = 38, EMPTY = 0xffffffff, DUMMY = 24 * 32;
    const align = 4 - (memSize & 3);
    // (the model's memory is kept for the next entry: a zip's entries all have the same settings, and the model does
    // not count on it being cleared)
    if (!sgUnppmd.mem || sgUnppmd.mem.length !== align + memSize) {
        sgUnppmd.mem = null;
        sgUnppmd.mem = new Uint8Array(align + memSize);
    }
    const mem = sgUnppmd.mem, dv = new DataView(mem.buffer), end = mem.length - 6;
    // (bytes read straight from the buffer; a pointer that a damaged entry sends past its end fails the 16 and 32 bit
    // reads, and the searches below stop at the end)
    const u8 = a => mem[a], w8 = (a, v) => { mem[a] = v; };
    const u16 = a => dv.getUint16(a, true), w16 = (a, v) => dv.setUint16(a, v, true);
    const u32 = a => dv.getUint32(a, true), w32 = (a, v) => dv.setUint32(a, v, true);
    // a state: symbol, freq, successor; a context: number of states - 1, flags, sum of freqs, states, suffix (one state
    // in place of the last three when it has only one)
    const freq = s => mem[s + 1], succ = s => dv.getUint32(s + 2, true), setSucc = (s, v) => dv.setUint32(s + 2, v, true);
    const ns = c => mem[c], flags = c => mem[c + 1], summ = c => dv.getUint16(c + 2, true);
    const stats = c => dv.getUint32(c + 4, true), suffix = c => dv.getUint32(c + 8, true);
    const copyState = (to, from) => mem.copyWithin(to, from, from + 6);
    const swap = (a, b) => {
        for (let k = 0; k < 6; k++) {
            const t = mem[a + k];
            mem[a + k] = mem[b + k];
            mem[b + k] = t;
        }
    };
    const copy12 = (to, from, n) => mem.copyWithin(to, from, from + n * UNIT);
    // the state of symbol `sym` in context `c`
    const find = (c, sym) => {
        let s = stats(c);
        while (mem[s] !== sym) if ((s += 6) > end) fail("damaged data");
        return s;
    };
    const i2u = new Uint8Array(NI), u2i = new Uint8Array(128);
    for (let i = 0, k = 0; i < NI; i++) {
        let step = i >= 12 ? 4 : (i >> 2) + 1;
        do u2i[k++] = i; while (--step);
        i2u[i] = k;
    }
    const ns2bs = new Uint8Array(256), ns2i = new Uint8Array(260);
    ns2bs[1] = 2;
    ns2bs.fill(4, 2, 11);
    ns2bs.fill(6, 11);
    for (let i = 0; i < 5; i++) ns2i[i] = i;
    for (let i = 5, m = 5, k = 1; i < 260; i++) {
        ns2i[i] = m;
        if (--k === 0) k = ++m - 4;
    }
    const EXP_ESCAPE = [25, 14, 9, 7, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 2];
    const INIT_BIN_ESC = [0x3cdd, 0x1f3f, 0x59bf, 0x48f3, 0x64a1, 0x5abc, 0x6632, 0x6051];
    const freeList = new Uint32Array(NI), stamps = new Uint32Array(NI);
    const seeSumm = new Uint16Array(DUMMY + 1), seeShift = new Uint8Array(DUMMY + 1), seeCount = new Uint8Array(DUMMY + 1);
    const binSumm = new Uint16Array(25 * 64);
    let minCtx = 0, maxCtx = 0, found = 0, orderFall = 0, initEsc = 0, prevSuccess = 0, runLength = 0, initRL = 0;
    let glueCount = 0, loUnit = 0, hiUnit = 0, text = 0, unitsStart = 0;

    // the memory: units handed out from the top (contexts) and from below the free area (states), freed units kept in
    // lists by their size
    const insertNode = (node, indx) => {
        w32(node, EMPTY);
        w32(node + 4, freeList[indx]);
        w32(node + 8, i2u[indx]);
        freeList[indx] = node;
        stamps[indx]++;
    };
    const removeNode = indx => {
        const node = freeList[indx];
        freeList[indx] = u32(node + 4);
        stamps[indx]--;
        return node;
    };
    const splitBlock = (ptr, oldIndx, newIndx) => {
        const nu = i2u[oldIndx] - i2u[newIndx];
        ptr += i2u[newIndx] * UNIT;
        let i = u2i[nu - 1];
        if (i2u[i] !== nu) {
            const k = i2u[--i];
            insertNode(ptr + k * UNIT, nu - k - 1);
        }
        insertNode(ptr, i);
    };
    const glueFreeBlocks = () => {
        let head = 0, prev = -1;
        const setPrev = v => { if (prev < 0) head = v; else w32(prev, v); };
        glueCount = 1 << 13;
        stamps.fill(0);
        if (loUnit !== hiUnit) w32(loUnit, 0);
        for (let i = 0; i < NI; i++) {
            let next = freeList[i];
            freeList[i] = 0;
            while (next !== 0) {
                const node = next;
                if (u32(node + 8) !== 0) {
                    setPrev(next);
                    prev = node + 4;
                    for (;;) {
                        const node2 = node + u32(node + 8) * UNIT;
                        if (u32(node2) !== EMPTY) break;
                        w32(node + 8, u32(node + 8) + u32(node2 + 8));
                        w32(node2 + 8, 0);
                    }
                }
                next = u32(node + 4);
            }
        }
        setPrev(0);
        while (head !== 0) {
            let node = head;
            head = u32(node + 4);
            let nu = u32(node + 8);
            if (nu === 0) continue;
            for (; nu > 128; nu -= 128, node += 128 * UNIT) insertNode(node, NI - 1);
            let i = u2i[nu - 1];
            if (i2u[i] !== nu) {
                const k = i2u[--i];
                insertNode(node + k * UNIT, nu - k - 1);
            }
            insertNode(node, i);
        }
    };
    const allocUnitsRare = indx => {
        if (glueCount === 0) {
            glueFreeBlocks();
            if (freeList[indx] !== 0) return removeNode(indx);
        }
        let i = indx;
        do {
            if (++i === NI) {
                const bytes = i2u[indx] * UNIT;
                glueCount = (glueCount - 1) >>> 0;
                if (unitsStart - text > bytes) {
                    unitsStart -= bytes;
                    return unitsStart;
                }
                return 0;
            }
        } while (freeList[i] === 0);
        const r = removeNode(i);
        splitBlock(r, i, indx);
        return r;
    };
    const allocUnits = indx => {
        if (freeList[indx] !== 0) return removeNode(indx);
        const bytes = i2u[indx] * UNIT;
        if (bytes <= hiUnit - loUnit) {
            const r = loUnit;
            loUnit += bytes;
            return r;
        }
        return allocUnitsRare(indx);
    };
    const shrinkUnits = (old, oldNU, newNU) => {
        const i0 = u2i[oldNU - 1], i1 = u2i[newNU - 1];
        if (i0 === i1) return old;
        if (freeList[i1] !== 0) {
            const ptr = removeNode(i1);
            copy12(ptr, old, newNU);
            insertNode(old, i0);
            return ptr;
        }
        splitBlock(old, i0, i1);
        return old;
    };
    const freeUnits = (ptr, nu) => insertNode(ptr, u2i[nu - 1]);
    const specialFreeUnit = ptr => {
        if (ptr !== unitsStart) insertNode(ptr, 0);
        else unitsStart += UNIT;
    };
    const moveUnitsUp = (old, nu) => {
        const indx = u2i[nu - 1];
        if (old > unitsStart + 16 * 1024 || old > freeList[indx]) return old;
        const ptr = removeNode(indx);
        copy12(ptr, old, nu);
        if (old !== unitsStart) insertNode(old, indx);
        else unitsStart += i2u[indx] * UNIT;
        return ptr;
    };
    const expandTextArea = () => {
        const count = new Uint32Array(NI);
        if (loUnit !== hiUnit) w32(loUnit, 0);
        let node = unitsStart;
        for (; u32(node) === EMPTY; node += u32(node + 8) * UNIT) {
            w32(node, 0);
            count[u2i[u32(node + 8) - 1]]++;
        }
        unitsStart = node;
        for (let i = 0; i < NI; i++) {
            let at = -1;
            const get = () => (at < 0 ? freeList[i] : u32(at));
            const set = v => { if (at < 0) freeList[i] = v; else w32(at, v); };
            while (count[i] !== 0) {
                let n = get();
                while (u32(n) === 0) {
                    set(u32(n + 4));
                    n = get();
                    stamps[i]--;
                    if (--count[i] === 0) break;
                }
                at = n + 4;
            }
        }
    };
    const usedMemory = () => {
        let v = 0;
        for (let i = 0; i < NI; i++) v += stamps[i] * i2u[i];
        return (memSize - (hiUnit - loUnit) - (unitsStart - text) - v * UNIT) >>> 0;
    };

    // the model
    const restartModel = () => {
        freeList.fill(0);
        stamps.fill(0);
        text = align;
        hiUnit = text + memSize;
        loUnit = unitsStart = hiUnit - Math.floor(Math.floor(memSize / 8) / UNIT) * 7 * UNIT;
        glueCount = 0;
        orderFall = maxOrder;
        runLength = initRL = -(maxOrder < 12 ? maxOrder : 12) - 1;
        prevSuccess = 0;
        hiUnit -= UNIT;
        minCtx = maxCtx = hiUnit;
        w32(minCtx + 8, 0);
        w8(minCtx, 255);
        w8(minCtx + 1, 0);
        w16(minCtx + 2, 257);
        found = loUnit;
        loUnit += 128 * UNIT;
        w32(minCtx + 4, found);
        for (let i = 0; i < 256; i++) {
            const s = found + i * 6;
            w8(s, i);
            w8(s + 1, 1);
            setSucc(s, 0);
        }
        for (let i = 0, m = 0; m < 25; m++) {
            while (ns2i[i] === m) i++;
            for (let k = 0; k < 8; k++) {
                const val = 16384 - Math.floor(INIT_BIN_ESC[k] / (i + 1));
                for (let r = 0; r < 64; r += 8) binSumm[m * 64 + k + r] = val;
            }
        }
        for (let i = 0, m = 0; m < 24; m++) {
            while (ns2i[i + 3] === m + 3) i++;
            for (let k = 0; k < 32; k++) {
                seeShift[m * 32 + k] = 3;
                seeSumm[m * 32 + k] = (2 * i + 5) << 3;
                seeCount[m * 32 + k] = 7;
            }
        }
    };
    const refresh = (ctx, oldNU, scale) => {
        let i = ns(ctx);
        let s = shrinkUnits(stats(ctx), oldNU, (i + 2) >> 1);
        w32(ctx + 4, s);
        let fl = (flags(ctx) & (0x10 + 0x04 * scale)) + 0x08 * (u8(s) >= 0x40);
        let escFreq = summ(ctx) - freq(s);
        let f = (freq(s) + scale) >> scale;
        w8(s + 1, f);
        let sumFreq = f;
        do {
            s += 6;
            escFreq -= freq(s);
            f = (freq(s) + scale) >> scale;
            w8(s + 1, f);
            sumFreq += f;
            fl |= 0x08 * (u8(s) >= 0x40);
        } while (--i);
        w16(ctx + 2, sumFreq + (((escFreq + scale) >>> 0) >>> scale));
        w8(ctx + 1, fl);
    };
    const cutOff = (ctx, order) => {
        if (!ns(ctx)) {
            const s = ctx + 2;
            if (succ(s) >= unitsStart) {
                setSucc(s, order < maxOrder ? cutOff(succ(s), order + 1) : 0);
                if (succ(s) || order <= 9) return ctx;
            }
            specialFreeUnit(ctx);
            return 0;
        }
        const tmp = (ns(ctx) + 2) >> 1;
        w32(ctx + 4, moveUnitsUp(stats(ctx), tmp));
        let i = ns(ctx);
        for (let s = stats(ctx) + i * 6; s >= stats(ctx); s -= 6) {
            if (succ(s) < unitsStart) {
                const s2 = stats(ctx) + i-- * 6;
                setSucc(s, 0);
                swap(s, s2);
            } else setSucc(s, order < maxOrder ? cutOff(succ(s), order + 1) : 0);
        }
        if (i !== ns(ctx) && order) {
            w8(ctx, i);
            const s = stats(ctx);
            if (i < 0) {
                freeUnits(s, tmp);
                specialFreeUnit(ctx);
                return 0;
            }
            if (i === 0) {
                w8(ctx + 1, (flags(ctx) & 0x10) + 0x08 * (u8(s) >= 0x40));
                copyState(ctx + 2, s);
                freeUnits(s, tmp);
                w8(ctx + 3, (freq(ctx + 2) + 11) >> 3);
            } else refresh(ctx, tmp, summ(ctx) > 16 * i ? 1 : 0);
        }
        return ctx;
    };
    const restoreModel = c1 => {
        text = align;
        let c;
        for (c = maxCtx; c !== c1; c = suffix(c)) {
            const n = (ns(c) - 1) & 255;
            w8(c, n);
            if (n === 0) {
                const s = stats(c);
                w8(c + 1, (flags(c) & 0x10) + 0x08 * (u8(s) >= 0x40));
                copyState(c + 2, s);
                specialFreeUnit(s);
                w8(c + 3, (freq(c + 2) + 11) >> 3);
            } else refresh(c, (n + 3) >> 1, 0);
        }
        for (; c !== minCtx; c = suffix(c)) {
            if (!ns(c)) w8(c + 3, freq(c + 2) - (freq(c + 2) >> 1));
            else {
                w16(c + 2, summ(c) + 4);
                if (summ(c) > 128 + 4 * ns(c)) refresh(c, (ns(c) + 2) >> 1, 1);
            }
        }
        if (restore === 0 || usedMemory() < memSize >>> 1) restartModel();
        else {
            while (suffix(maxCtx)) maxCtx = suffix(maxCtx);
            do {
                cutOff(maxCtx, 0);
                expandTextArea();
            } while (usedMemory() > 3 * (memSize >>> 2));
            glueCount = 0;
            orderFall = maxOrder;
        }
    };
    const createSuccessors = (skip, s1, c) => {
        const upBranch = succ(found);
        const ps = [];
        if (!skip) ps.push(found);
        while (suffix(c)) {
            let s;
            c = suffix(c);
            if (s1) {
                s = s1;
                s1 = 0;
            } else if (ns(c) !== 0) {
                s = find(c, u8(found));
                if (freq(s) < MAX_FREQ - 9) {
                    w8(s + 1, freq(s) + 1);
                    w16(c + 2, summ(c) + 1);
                }
            } else {
                s = c + 2;
                w8(s + 1, freq(s) + ((ns(suffix(c)) === 0) & (freq(s) < 24)));
            }
            const successor = succ(s);
            if (successor !== upBranch) {
                c = successor;
                if (ps.length === 0) return c;
                break;
            }
            ps.push(s);
        }
        const upSym = u8(upBranch), upSucc = upBranch + 1;
        const fl = 0x10 * (u8(found) >= 0x40) + 0x08 * (upSym >= 0x40);
        let upFreq;
        if (ns(c) === 0) upFreq = freq(c + 2);
        else {
            const s = find(c, upSym);
            const cf = (freq(s) - 1) >>> 0, s0 = (summ(c) - ns(c) - cf) >>> 0;
            upFreq = 1 + (2 * cf <= s0 ? (5 * cf > s0 ? 1 : 0) : Math.floor(((cf + 2 * s0 - 3) >>> 0) / s0));
        }
        do {
            let c1;
            if (hiUnit !== loUnit) c1 = hiUnit -= UNIT;
            else if (freeList[0] !== 0) c1 = removeNode(0);
            else {
                c1 = allocUnitsRare(0);
                if (!c1) return 0;
            }
            w8(c1, 0);
            w8(c1 + 1, fl);
            w8(c1 + 2, upSym);
            w8(c1 + 3, upFreq);
            w32(c1 + 4, upSucc);
            w32(c1 + 8, c);
            setSucc(ps.pop(), c1);
            c = c1;
        } while (ps.length);
        return c;
    };
    const reduceOrder = (s1, c) => {
        let s = 0;
        const c1 = c, upBranch = text, sym = u8(found);
        setSucc(found, upBranch);
        orderFall++;
        for (;;) {
            if (s1) {
                c = suffix(c);
                s = s1;
                s1 = 0;
            } else {
                if (!suffix(c)) return c;
                c = suffix(c);
                if (ns(c)) {
                    s = find(c, sym);
                    if (freq(s) < MAX_FREQ - 9) {
                        w8(s + 1, freq(s) + 2);
                        w16(c + 2, summ(c) + 2);
                    }
                } else {
                    s = c + 2;
                    w8(s + 1, freq(s) + (freq(s) < 32));
                }
            }
            if (succ(s)) break;
            setSucc(s, upBranch);
            orderFall++;
        }
        if (succ(s) <= upBranch) {
            const s2 = found;
            found = s;
            setSucc(s, createSuccessors(false, 0, c));
            found = s2;
        }
        if (orderFall === 1 && c1 === maxCtx) {
            setSucc(found, succ(s));
            text--;
        }
        return succ(s);
    };
    const updateModel = () => {
        let fSuccessor = succ(found);
        const fFreq = freq(found), fSymbol = u8(found);
        let s = 0, c;
        if (fFreq < MAX_FREQ / 4 && suffix(minCtx) !== 0) {
            c = suffix(minCtx);
            if (ns(c) === 0) {
                s = c + 2;
                if (freq(s) < 32) w8(s + 1, freq(s) + 1);
            } else {
                s = find(c, fSymbol);
                if (s !== stats(c)) {
                    if (freq(s) >= freq(s - 6)) {
                        swap(s, s - 6);
                        s -= 6;
                    }
                }
                if (freq(s) < MAX_FREQ - 9) {
                    w8(s + 1, freq(s) + 2);
                    w16(c + 2, summ(c) + 2);
                }
            }
        }
        c = maxCtx;
        if (orderFall === 0 && fSuccessor) {
            const cs = createSuccessors(true, s, minCtx);
            if (cs === 0) {
                setSucc(found, 0);
                restoreModel(c);
            } else {
                setSucc(found, cs);
                maxCtx = cs;
            }
            return;
        }
        w8(text++, fSymbol);
        let successor = text;
        if (text >= unitsStart) {
            restoreModel(c);
            return;
        }
        if (!fSuccessor) {
            const cs = reduceOrder(s, minCtx);
            if (cs === 0) {
                restoreModel(c);
                return;
            }
            fSuccessor = cs;
        } else if (fSuccessor < unitsStart) {
            const cs = createSuccessors(false, s, minCtx);
            if (cs === 0) {
                restoreModel(c);
                return;
            }
            fSuccessor = cs;
        }
        if (--orderFall === 0) {
            successor = fSuccessor;
            if (maxCtx !== minCtx) text--;
        }
        const ns0 = ns(minCtx), s0 = (summ(minCtx) - ns0 - fFreq) >>> 0, flag = 0x08 * (fSymbol >= 0x40);
        for (; c !== minCtx; c = suffix(c)) {
            const ns1 = ns(c);
            if (ns1 !== 0) {
                if (ns1 & 1) {
                    const oldNU = (ns1 + 1) >> 1, i = u2i[oldNU - 1];
                    if (i !== u2i[oldNU]) {
                        const ptr = allocUnits(i + 1);
                        if (!ptr) {
                            restoreModel(c);
                            return;
                        }
                        const old = stats(c);
                        copy12(ptr, old, oldNU);
                        insertNode(old, i);
                        w32(c + 4, ptr);
                    }
                }
                w16(c + 2, summ(c) + (3 * ns1 + 1 < ns0 ? 1 : 0));
            } else {
                const s2 = allocUnits(0);
                if (!s2) {
                    restoreModel(c);
                    return;
                }
                copyState(s2, c + 2);
                w32(c + 4, s2);
                w8(s2 + 1, freq(s2) < MAX_FREQ / 4 - 1 ? freq(s2) << 1 : MAX_FREQ - 4);
                w16(c + 2, freq(s2) + initEsc + (ns0 > 2 ? 1 : 0));
            }
            let cf = 2 * fFreq * (summ(c) + 6);
            const sf = s0 + summ(c);
            if (cf < 6 * sf) {
                cf = 1 + (cf > sf) + (cf >= 4 * sf);
                w16(c + 2, summ(c) + 4);
            } else {
                cf = 4 + (cf > 9 * sf) + (cf > 12 * sf) + (cf > 15 * sf);
                w16(c + 2, summ(c) + cf);
            }
            const s2 = stats(c) + (ns1 + 1) * 6;
            setSucc(s2, successor);
            w8(s2, fSymbol);
            w8(s2 + 1, cf);
            w8(c + 1, flags(c) | flag);
            w8(c, ns1 + 1);
        }
        maxCtx = minCtx = fSuccessor;
    };
    const rescale = () => {
        const st = stats(minCtx);
        let s = found;
        {
            const tmp = mem.slice(s, s + 6);
            mem.copyWithin(st + 6, st, s);
            mem.set(tmp, st);
        }
        s = st;
        let escFreq = summ(minCtx) - freq(s);
        const adder = orderFall !== 0 ? 1 : 0;
        w8(s + 1, (freq(s) + 4 + adder) >> 1);
        let sumFreq = freq(s);
        let i = ns(minCtx);
        do {
            s += 6;
            escFreq -= freq(s);
            w8(s + 1, (freq(s) + adder) >> 1);
            sumFreq += freq(s);
            if (freq(s) > freq(s - 6)) {
                let s1 = s;
                const tmp = mem.slice(s1, s1 + 6);
                do {
                    copyState(s1, s1 - 6);
                    s1 -= 6;
                } while (s1 !== st && tmp[1] > freq(s1 - 6));
                mem.set(tmp, s1);
            }
        } while (--i);
        if (freq(s) === 0) {
            const numStats = ns(minCtx);
            do {
                i++;
                s -= 6;
            } while (freq(s) === 0);
            escFreq += i;
            w8(minCtx, numStats - i);
            if (ns(minCtx) === 0) {
                const tmp = mem.slice(st, st + 6);
                tmp[1] = Math.min(MAX_FREQ / 3 | 0, Math.floor((2 * tmp[1] + escFreq - 1) / escFreq));
                insertNode(st, u2i[((numStats + 2) >> 1) - 1]);
                w8(minCtx + 1, (flags(minCtx) & 0x10) + 0x08 * (tmp[0] >= 0x40));
                found = minCtx + 2;
                mem.set(tmp, found);
                return;
            }
            const n0 = (numStats + 2) >> 1, n1 = (ns(minCtx) + 2) >> 1;
            if (n0 !== n1) w32(minCtx + 4, shrinkUnits(st, n0, n1));
            w8(minCtx + 1, flags(minCtx) & ~0x08);
            s = stats(minCtx);
            w8(minCtx + 1, flags(minCtx) | (0x08 * (u8(s) >= 0x40)));
            i = ns(minCtx);
            do {
                s += 6;
                w8(minCtx + 1, flags(minCtx) | (0x08 * (u8(s) >= 0x40)));
            } while (--i);
        }
        w16(minCtx + 2, sumFreq + escFreq - (escFreq >> 1));
        w8(minCtx + 1, flags(minCtx) | 4);
        found = stats(minCtx);
    };
    const nextContext = () => {
        const c = succ(found);
        if (orderFall === 0 && c >= unitsStart) minCtx = maxCtx = c;
        else {
            updateModel();
            minCtx = maxCtx;
        }
    };

    // the range decoder
    let pos = 2, low = 0, range = 0xffffffff, code = 0;
    const next = () => (pos < src.length ? src[pos++] : (pos++, 0));
    for (let i = 0; i < 4; i++) code = ((code << 8) | next()) >>> 0;
    if (code === 0xffffffff) fail("a bad start of the stream");
    const threshold = total => {
        range = Math.floor(range / total);
        return Math.floor(code / range);
    };
    const decode = (start, sz) => {
        start *= range;
        low = (low + start) >>> 0;
        code = (code - start) >>> 0;
        range = (range * sz) >>> 0;
        for (;;) {
            if (((low ^ ((low + range) >>> 0)) >>> 0) >= 0x1000000) {
                if (range >= 0x8000) break;
                range = -low & 0x7fff;
            }
            code = ((code << 8) | next()) >>> 0;
            range = (range << 8) >>> 0;
            low = (low << 8) >>> 0;
        }
    };
    restartModel();
    seeShift[DUMMY] = 7;
    seeCount[DUMMY] = 64;

    const mask = new Uint8Array(256), ps = new Uint32Array(256);
    const symbol = () => {
        if (ns(minCtx) !== 0) {
            let s = stats(minCtx);
            const count = threshold(summ(minCtx));
            let hiCnt = freq(s);
            if (count < hiCnt) {
                decode(0, hiCnt);
                found = s;
                const sym = u8(s);
                // Update1_0
                prevSuccess = 2 * freq(found) >= summ(minCtx) ? 1 : 0;
                runLength += prevSuccess;
                w16(minCtx + 2, summ(minCtx) + 4);
                w8(found + 1, freq(found) + 4);
                if (freq(found) > MAX_FREQ) rescale();
                nextContext();
                return sym;
            }
            prevSuccess = 0;
            let i = ns(minCtx);
            do {
                s += 6;
                if ((hiCnt += freq(s)) > count) {
                    decode(hiCnt - freq(s), freq(s));
                    found = s;
                    const sym = u8(s);
                    // Update1
                    w8(s + 1, freq(s) + 4);
                    w16(minCtx + 2, summ(minCtx) + 4);
                    if (freq(s) > freq(s - 6)) {
                        swap(s, s - 6);
                        found = s -= 6;
                        if (freq(s) > MAX_FREQ) rescale();
                    }
                    nextContext();
                    return sym;
                }
            } while (--i);
            if (count >= summ(minCtx)) return -2;
            decode(hiCnt, summ(minCtx) - hiCnt);
            mask.fill(0xff);
            mask[u8(s)] = 0;
            i = ns(minCtx);
            do mask[u8(s -= 6)] = 0; while (--i);
        } else {
            const one = minCtx + 2;
            const bi = ns2i[freq(one) - 1] * 64 + ns2bs[ns(suffix(minCtx))] + prevSuccess + flags(minCtx) + ((runLength >> 26) & 0x20);
            const prob = binSumm[bi];
            range >>>= 14;
            if (Math.floor(code / range) < prob) {
                decode(0, prob);
                binSumm[bi] = prob + 128 - ((prob + 32) >> 7);
                found = one;
                const sym = u8(one);
                // UpdateBin
                w8(one + 1, freq(one) + (freq(one) < 196));
                prevSuccess = 1;
                runLength++;
                nextContext();
                return sym;
            }
            decode(prob, 16384 - prob);
            binSumm[bi] = prob - ((prob + 32) >> 7);
            initEsc = EXP_ESCAPE[binSumm[bi] >> 10];
            mask.fill(0xff);
            mask[u8(one)] = 0;
            prevSuccess = 0;
        }
        for (;;) {
            const numMasked = ns(minCtx);
            do {
                orderFall++;
                if (!suffix(minCtx)) return -1;
                minCtx = suffix(minCtx);
            } while (ns(minCtx) === numMasked);
            let hiCnt = 0, s = stats(minCtx), i = 0;
            const num = ns(minCtx) - numMasked;
            do {
                if (mask[mem[s]]) {
                    hiCnt += mem[s + 1];
                    ps[i++] = s;
                }
                if ((s += 6) > end) fail("damaged data");
            } while (i !== num);
            // MakeEscFreq
            let see, freqSum;
            if (ns(minCtx) !== 0xff) {
                see = (ns2i[ns(minCtx) + 2] - 3) * 32 + (summ(minCtx) > 11 * (ns(minCtx) + 1) ? 1 : 0) +
                    2 * (2 * ns(minCtx) < ns(suffix(minCtx)) + numMasked ? 1 : 0) + flags(minCtx);
                const r = seeSumm[see] >> seeShift[see];
                seeSumm[see] -= r;
                freqSum = r + (r === 0 ? 1 : 0);
            } else {
                see = DUMMY;
                freqSum = 1;
            }
            freqSum += hiCnt;
            const count = threshold(freqSum);
            if (count < hiCnt) {
                let k = 0;
                for (hiCnt = 0; (hiCnt += freq(ps[k])) <= count; k++);
                s = ps[k];
                decode(hiCnt - freq(s), freq(s));
                if (seeShift[see] < 7 && --seeCount[see] === 0) {
                    seeSumm[see] <<= 1;
                    seeCount[see] = 3 << seeShift[see]++;
                }
                found = s;
                const sym = u8(s);
                // Update2
                w16(minCtx + 2, summ(minCtx) + 4);
                w8(s + 1, freq(s) + 4);
                if (freq(s) > MAX_FREQ) rescale();
                runLength = initRL;
                updateModel();
                minCtx = maxCtx;
                return sym;
            }
            if (count >= freqSum) return -2;
            decode(hiCnt, freqSum - hiCnt);
            seeSumm[see] += freqSum;
            do mask[u8(ps[--i])] = 0; while (i !== 0);
        }
    };
    const out = new Uint8Array(size);
    for (let o = 0; o < size; o++) {
        const sym = symbol();
        if (sym < 0) fail(sym === -1 ? "the data ends early" : "damaged data");
        out[o] = sym;
    }
    return out;
}

// An entry's name: UTF-8 when its flag says so or it reads as UTF-8 (a Mac's and 7-Zip's zips), else the DOS code page
// (CP437) old Windows zippers wrote, or the Info-ZIP Unicode path in its extra field when it has one.
const SG_CP437 = "ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■ ";
function sgZipName(raw, flags, extra) {
    for (let x = 0; x + 4 <= extra.length;) {
        const id = extra[x] | (extra[x + 1] << 8), n = extra[x + 2] | (extra[x + 3] << 8);
        if (id === 0x7075 && n >= 5 && x + 4 + n <= extra.length && extra[x + 4] === 1) {
            const crc = (extra[x + 5] | (extra[x + 6] << 8) | (extra[x + 7] << 16) | (extra[x + 8] << 24)) >>> 0;
            if (crc === sgCrc32(raw)) return new TextDecoder().decode(extra.subarray(x + 9, x + 4 + n));
        }
        x += 4 + n;
    }
    if (flags & 0x800) return new TextDecoder().decode(raw);
    try {
        return new TextDecoder("utf-8", { fatal: true }).decode(raw);
    } catch (e) {
        let s = "";
        for (const b of raw) s += b < 128 ? String.fromCharCode(b) : SG_CP437[b - 128];
        return s;
    }
}
// A name with forward slashes, no leading "/" or "./" and no empty parts (a folder keeps its trailing "/").
function sgZipPath(name) {
    const dir = /[\\/]$/.test(name);
    const parts = name.replace(/\\/g, "/").split("/").filter(s => s !== "" && s !== ".");
    return parts.join("/") + (dir && parts.length ? "/" : "");
}

// `size` bytes at `offset` of a Blob (of a File only that part is read: a zip or a CD image is never held whole).
async function sgSlice(blob, offset, size) {
    return new Uint8Array(await blob.slice(offset, offset + size).arrayBuffer());
}
function sgLe64(v, at) {
    return v.getUint32(at, true) + v.getUint32(at + 4, true) * 4294967296;
}
// A zip's directory, read from the end of the Blob: [{ name, dir, flags, method, crc, csize, usize, local, nlen, xlen }].
// Zip64 (archives over 4 GB or 65535 files, and some zippers' entries over 4 GB) is read too.
async function sgZipEntries(blob) {
    const tailLen = Math.min(blob.size, 22 + 0xffff);
    const tail = await sgSlice(blob, blob.size - tailLen, tailLen);
    const t = new DataView(tail.buffer);
    let e = tail.length - 22;
    while (e >= 0 && t.getUint32(e, true) !== 0x06054b50) e--;
    if (e < 0) throw new Error("not a zip file");
    let count = t.getUint16(e + 10, true), cdSize = t.getUint32(e + 12, true), cdAt = t.getUint32(e + 16, true);
    if ((count === 0xffff || cdSize === 0xffffffff || cdAt === 0xffffffff) && e >= 20 && t.getUint32(e - 20, true) === 0x07064b50) {
        const z = await sgSlice(blob, sgLe64(t, e - 20 + 8), 56);
        const zv = new DataView(z.buffer);
        if (z.length < 56 || zv.getUint32(0, true) !== 0x06064b50) throw new Error("damaged Zip64 record");
        count = sgLe64(zv, 32);
        cdSize = sgLe64(zv, 40);
        cdAt = sgLe64(zv, 48);
    }
    if (cdAt + cdSize > blob.size) throw new Error("the zip's directory is past its end (a download cut short?)");
    const cd = await sgSlice(blob, cdAt, cdSize);
    const v = new DataView(cd.buffer);
    const out = [];
    let p = 0;
    for (let i = 0; i < count; i++) {
        if (p + 46 > cd.length || v.getUint32(p, true) !== 0x02014b50) throw new Error("damaged zip directory");
        const nlen = v.getUint16(p + 28, true), xlen = v.getUint16(p + 30, true), clen = v.getUint16(p + 32, true);
        const flags = v.getUint16(p + 8, true);
        const extra = cd.subarray(p + 46 + nlen, p + 46 + nlen + xlen);
        const name = sgZipPath(sgZipName(cd.subarray(p + 46, p + 46 + nlen), flags, extra));
        const en = {
            name, dir: name.endsWith("/"), flags, method: v.getUint16(p + 10, true), crc: v.getUint32(p + 16, true),
            csize: v.getUint32(p + 20, true), usize: v.getUint32(p + 24, true), local: v.getUint32(p + 42, true), nlen, xlen,
        };
        // the Zip64 extra field: the values that did not fit, in this order
        for (let x = 0; x + 4 <= extra.length;) {
            const id = extra[x] | (extra[x + 1] << 8), n = extra[x + 2] | (extra[x + 3] << 8);
            if (id === 1) {
                const xv = new DataView(extra.buffer, extra.byteOffset + x + 4, Math.min(n, extra.length - x - 4));
                let o = 0;
                for (const k of ["usize", "csize", "local"]) {
                    if (en[k] === 0xffffffff && o + 8 <= xv.byteLength) {
                        en[k] = sgLe64(xv, o);
                        o += 8;
                    }
                }
            }
            x += 4 + n;
        }
        out.push(en);
        p += 46 + nlen + xlen + clen;
    }
    return out;
}
// Where an entry's (compressed) bytes start in the Blob, from its local header.
async function sgZipDataAt(blob, e) {
    const h = await sgSlice(blob, e.local, 30);
    const v = new DataView(h.buffer);
    if (h.length < 30 || v.getUint32(0, true) !== 0x04034b50) throw new Error("damaged zip entry " + e.name);
    return e.local + 30 + v.getUint16(26, true) + v.getUint16(28, true);
}
// An entry's bytes as stored in the zip (one read with the local header in most cases).
async function sgZipRaw(blob, e) {
    let d = await sgSlice(blob, e.local, 30 + e.nlen + e.xlen + 256 + e.csize);
    const v = new DataView(d.buffer);
    if (d.length < 30 || v.getUint32(0, true) !== 0x04034b50) throw new Error("damaged zip entry " + e.name);
    const start = 30 + v.getUint16(26, true) + v.getUint16(28, true);
    if (start + e.csize > d.length) d = await sgSlice(blob, e.local, start + e.csize);
    if (start + e.csize > d.length) throw new Error("zip entry " + e.name + " is cut short");
    return d.subarray(start, start + e.csize);
}
// Plain deflate: the browser's own DecompressionStream where it has one (fast), else, or when it fails or gives other
// bytes than the zip says (an old Safari's), the page's own decoder. `stat.fallback` counts the second.
async function sgUndeflate(data, e, stat) {
    if (typeof DecompressionStream !== "undefined") {
        try {
            const out = new Uint8Array(await new Response(new Blob([data]).stream().pipeThrough(new DecompressionStream("deflate-raw"))).arrayBuffer());
            if (out.length === e.usize) return out;
        } catch (err) {}
    }
    if (stat) stat.fallback = (stat.fallback || 0) + 1;
    return sgInflate(data, e.usize, false);
}
// The methods unpacked here, by zip number: (compressed bytes, entry, stats) => bytes.
const SG_UNPACK = {
    0: data => data.slice(),
    8: sgUndeflate,
    9: (data, e) => sgInflate(data, e.usize, true),
    12: (data, e) => sgBunzip2(data, e.usize),
    14: (data, e) => sgUnlzma(data, e.usize),
    98: (data, e) => sgUnppmd(data, e.usize),
};
// An entry unpacked, or a string saying why it cannot be. A checksum that does not match is counted in `stat.crc` (the
// file is kept: a sound with a damaged end still plays).
async function sgZipRead(blob, e, stat = null) {
    if (e.flags & 1) return "encrypted"; // a password-protected entry: its bytes are scrambled
    const unpack = SG_UNPACK[e.method];
    if (!unpack) return sgMethodName(e.method) + ", which the page cannot unpack";
    if (e.usize > SG_ZIP_MAX) return "too big (" + Math.round(e.usize / 1048576) + " MB)";
    if (e.usize === 0) return new Uint8Array(0);
    let out;
    try {
        out = await unpack(await sgZipRaw(blob, e), e, stat);
    } catch (err) {
        console.warn("zip: " + e.name + ": " + err.message);
        return "damaged (" + sgMethodName(e.method) + ")";
    }
    if (sgCrc32(out) !== e.crc && e.method === 8 && typeof DecompressionStream !== "undefined") {
        // the browser's decoder gave other bytes (or the entry is damaged): the page's own decides
        try {
            out = sgInflate(await sgZipRaw(blob, e), e.usize, false);
            if (stat) stat.fallback = (stat.fallback || 0) + 1;
        } catch (err) {}
    }
    if (stat && sgCrc32(out) !== e.crc) {
        stat.crc = (stat.crc || 0) + 1;
        console.warn("zip: " + e.name + ": its checksum does not match (kept)");
    }
    return out;
}
// Calls onEntry for each file it can unpack; the ones it cannot are added to `skipped` as [name, why]. Returns how many.
// `src` is a Blob (a picked File, read a part at a time) or an ArrayBuffer. `stat` gathers what the unpacking met: the
// methods of the files ({ method name: count }), deflate entries the browser could not unpack, checksums that did not match.
async function sgUnzip(src, onEntry, skipped = [], list = null, stat = {}) {
    const blob = src instanceof Blob ? src : new Blob([src]);
    const entries = list || await sgZipEntries(blob);
    stat.methods = stat.methods || {};
    for (let i = 0; i < entries.length; i++) {
        const e = entries[i];
        if (e.dir || sgSkip(e.name.toLowerCase())) continue;
        const m = sgMethodName(e.method);
        stat.methods[m] = (stat.methods[m] || 0) + 1;
        const bytes = await sgZipRead(blob, e, stat);
        if (typeof bytes === "string") {
            if (skipped.length < 5) console.warn("zip: cannot unpack " + e.name + " (" + bytes + ")");
            skipped.push([e.name, bytes]);
            continue;
        }
        await onEntry(e.name, bytes, i, entries.length);
    }
    if (stat.fallback) console.log("zip: " + sgPlural(stat.fallback, "deflated file") + " unpacked by the page's own decoder (the browser's could not)");
    return skipped.length;
}

// The sound files of the game folder ([path, bytes], "game/...") in a coding the game cannot play: { n, example }.
function sgSoundCodings(files) {
    const bad = { n: 0, example: "" };
    for (const [p, bytes] of files) {
        const k = sgKey(p);
        if (!(k.startsWith("game/sounds/") || k.startsWith("game/simsfx/")) || !sgIsSound(k)) continue;
        const why = sgSoundCoding(bytes);
        if (why !== null && bad.n++ === 0) bad.example = why;
    }
    return bad;
}

// What the player's files hold, said in large text on the start page (and on it again next time) and over the game once it
// starts (an iPad shows no console): the version, the sounds found, used and left out (and why), the art, the music and
// the Sims' voices. A copy with empty Sounds and SimsFX folders (some downloaded copies, or a zip whose sounds could not be
// unpacked) plays the free OpenSGolf sounds, which players took for a fault (issue #10). `paths` are the game folder's
// ("game/..."); `skipped` is sgUnzip's; `found` the sound files the zip or folder held anywhere (null: not known, a start
// from the files kept in the browser); `bad` the sound files in a coding the game cannot play, as { n, example }
// (sgSoundCodings). Returns { text, problem }.
function sgFilesSummary(paths, skipped = [], found = null, bad = null) {
    let art = 0, sounds = 0, music = 0, voices = 0;
    for (const path of paths) {
        const k = sgKey(path);
        const sound = sgIsSound(k);
        if (k.startsWith("game/flics/") || k.startsWith("game/interface/")) art++;
        else if (k.startsWith("game/simsfx/") && sound) voices++;
        else if (k.startsWith("game/sounds/") && sound) {
            sounds++;
            if (/\bmusic\//.test(k)) music++;
        }
    }
    const skippedSounds = skipped.filter(([n]) => sgIsSound(n));
    const why = new Map();
    for (const [, w] of skippedSounds) why.set(w, (why.get(w) || 0) + 1);
    const used = sounds + voices;
    const say = ["OpenSGolf " + sgVersion() + ". Sounds: " + (found === null ? "" : "found " + found + ", ") + "used " + used +
        (found === null ? "" : ", skipped " + skippedSounds.length +
            (why.size ? " (" + Array.from(why, ([w, n]) => n + " " + w).join("; ") + ")" : "")) +
        ". Art " + (art > 0 ? "✓" : "✗ (none)") + ", music: " + music + ", Sims' voices: " + voices + "."];
    let problem = false;
    const others = skipped.length - skippedSounds.length;
    if (skipped.length > 0) {
        problem = true;
        if (others > 0) say.push(sgPlural(others, "other file") + " could not be unpacked either.");
        say.push("Zip the folder again with normal (Deflate) compression, or pick the folder itself.");
    }
    if (used === 0) {
        problem = true;
        say.push(skippedSounds.length ? "So there are no sounds from your files: the free OpenSGolf sounds play instead."
            : "Sounds: none in your files (the Sounds and SimsFX folders are empty), so the free OpenSGolf sounds play. " +
              "For the original sounds, use the SimGolf CD or its .iso, or a full installed copy.");
    } else if (sounds === 0) {
        problem = true;
        say.push("Your files have no Sounds folder (only the Sims' voices): the free OpenSGolf sounds play for the rest. " +
            "For the original sounds, use the SimGolf CD or its .iso, or a full installed copy.");
    } else if (used < 100) {
        // a full copy has about 320 sound files; a downloaded copy with the sounds left out keeps a stray one or two
        problem = true;
        say.push("Only " + sgPlural(used, "sound") + " in your files (some downloaded copies leave the sounds out): the free " +
            "OpenSGolf sounds play for the rest. For the original sounds, use the SimGolf CD or its .iso, or a full installed copy.");
    }
    if (bad && bad.n > 0) {
        problem = true;
        say.push(bad.n + " of your sounds use a format OpenSGolf can't play yet (e.g. " + bad.example + "): the free OpenSGolf " +
            "sounds play for those. A copy of the game with its original sounds (the CD, or a full installed copy) plays them all.");
    }
    return { text: say.join(" "), problem };
}

if (typeof module !== "undefined") {
    module.exports = { sgKey, sgSkip, sgRootOf, sgStraySound, sgPlace, sgZipEntries, sgZipRead, sgUnzip, sgInflate, sgInflateInto,
        sgBunzip2, sgUnlzma, sgUnppmd, sgZipName, sgZipPath, sgCrc32, sgFilesSummary, sgVersion, sgSoundCoding,
        sgSoundCodings, sgPortraitAsk, sgCanvasRect, sgSafeInsets, sgSoundAction };
}
