// Tests of the page's zip reading and file placement (web/files.js), run by web/build.sh: `node web/test.js`. The method
// zips were made with 7-Zip (`7z a -tzip -mm=BZip2|LZMA|PPMd|Deflate64`) and Python's zipfile (ZIP_BZIP2, ZIP_LZMA) from
// made-up files laid out like a game folder; the layouts are written here.
"use strict";
const assert = require("assert");
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");
const f = require("./files.js");

const data = path.join(__dirname, "../crates/sg-core/tests/data/");
// (what the unzip says about the files it meets is for the page's console, not for the test's output)
console.warn = () => {};
const say = console.log;
console.log = (...a) => { if (!/^zip: /.test(a[0])) say(...a); };
let failed = 0;
const tests = [];
const test = (name, fn) => tests.push([name, fn]);

// Unpacks a zip (bytes); returns { files: { name: bytes }, skipped, stat }.
async function unzip(bytes) {
    const files = {}, skipped = [], stat = {};
    await f.sgUnzip(new Blob([bytes]), async (name, b) => { files[name] = b; }, skipped, null, stat);
    return { files, skipped, stat };
}

// A zip written here: entries [{ name (string or bytes), data, flags, method, extra, zip64 }], stored unless `method`
// and `raw` (the compressed bytes) say otherwise; `zip64end` writes the Zip64 end records with the 32-bit fields full.
function zip(entries, zip64end = false) {
    const parts = [], central = [];
    let off = 0;
    const le = (n, v) => {
        const b = Buffer.alloc(n);
        if (n === 8) b.writeBigUInt64LE(BigInt(v));
        else b.writeUIntLE(v, 0, n);
        return b;
    };
    for (const e of entries) {
        const name = typeof e.name === "string" ? Buffer.from(e.name, "utf8") : Buffer.from(e.name);
        const body = Buffer.from(e.raw || e.data);
        const crc = f.sgCrc32(e.data);
        const method = e.method || 0, flags = e.flags || 0;
        let extra = Buffer.from(e.extra || []);
        let csize = body.length, usize = e.data.length, local = off;
        if (e.zip64) {
            extra = Buffer.concat([extra, le(2, 1), le(2, 24), le(8, usize), le(8, csize), le(8, local)]);
            csize = usize = local = 0xffffffff;
        }
        const lh = Buffer.concat([le(4, 0x04034b50), le(2, 20), le(2, flags), le(2, method), le(4, 0), le(4, crc),
            le(4, body.length), le(4, e.data.length), le(2, name.length), le(2, 0), name]);
        const ch = Buffer.concat([le(4, 0x02014b50), le(2, 20), le(2, 20), le(2, flags), le(2, method), le(4, 0), le(4, crc),
            le(4, csize), le(4, usize), le(2, name.length), le(2, extra.length), le(2, 0), le(2, 0), le(2, 0), le(4, 0),
            le(4, local), name, extra]);
        parts.push(lh, body);
        central.push(ch);
        off += lh.length + body.length;
    }
    const cd = Buffer.concat(central);
    const tail = [];
    if (zip64end) {
        const z = Buffer.concat([le(4, 0x06064b50), le(8, 44), le(2, 45), le(2, 45), le(4, 0), le(4, 0), le(8, entries.length),
            le(8, entries.length), le(8, cd.length), le(8, off)]);
        tail.push(z, le(4, 0x07064b50), le(4, 0), le(8, off + cd.length), le(4, 1));
        tail.push(le(4, 0x06054b50), le(4, 0), le(2, 0xffff), le(2, 0xffff), le(4, 0xffffffff), le(4, 0xffffffff), le(2, 0));
    } else {
        tail.push(le(4, 0x06054b50), le(4, 0), le(2, entries.length), le(2, entries.length), le(4, cd.length), le(4, off), le(2, 0));
    }
    return Buffer.concat([...parts, cd, ...tail]);
}
const bytes = s => new Uint8Array(Buffer.from(s));
// Where the files of a zip go in the game folder: { path under the game folder (lower case): name in the zip }.
async function placed(z) {
    const { files } = await unzip(z);
    const names = Object.keys(files);
    const out = {};
    for (const [i, rel] of f.sgPlace(names)) out[f.sgKey(rel)] = names[i];
    return out;
}

// --- compression methods ----------------------------------------------------------------------------------------------------
for (const [file, method] of [["bzip2.zip", "BZip2"], ["bzip2-7z.zip", "BZip2"], ["lzma.zip", "LZMA"], ["lzma-7z.zip", "LZMA"],
    ["ppmd-7z.zip", "PPMd"], ["deflate64.zip", "Deflate64"], ["game64.zip", "Deflate64"]]) {
    test("unpacks " + file + " (" + method + ")", async () => {
        const { files, skipped, stat } = await unzip(fs.readFileSync(data + file));
        assert.deepStrictEqual(skipped, []);
        assert.ok(stat.methods[method] > 0, JSON.stringify(stat.methods));
        assert.ok(!stat.crc, "every checksum matches");
        assert.ok(Object.keys(files).length >= 1);
        if (file.startsWith("bzip2") || file.startsWith("lzma") || file.startsWith("ppmd")) {
            // (the same made-up game folder in each: a long file over several BZip2 blocks, a picture, two sounds)
            assert.deepStrictEqual(Object.keys(files).sort(), ["SimGolf/Data/long.txt", "SimGolf/Flics/title.fli",
                "SimGolf/SimsFX/Male/hello.wav", "SimGolf/Sounds/Effects/Splash.wav"]);
            assert.strictEqual(files["SimGolf/Data/long.txt"].length, 265000);
        }
    });
}

test("all the method zips unpack to the same files", async () => {
    const want = (await unzip(fs.readFileSync(data + "bzip2.zip"))).files;
    for (const file of ["bzip2-7z.zip", "lzma.zip", "lzma-7z.zip", "ppmd-7z.zip"]) {
        const got = (await unzip(fs.readFileSync(data + file))).files;
        for (const k of Object.keys(want)) assert.ok(Buffer.from(want[k]).equals(Buffer.from(got[k])), file + ": " + k);
    }
});

test("a method the page does not know is skipped with its name", async () => {
    const z = zip([{ name: "SimGolf/Flics/a.fli", data: bytes("art") }, { name: "SimGolf/Sounds/x.wav", data: bytes("RIFF"), method: 93 }]);
    const { files, skipped } = await unzip(z);
    assert.deepStrictEqual(Object.keys(files), ["SimGolf/Flics/a.fli"]);
    assert.deepStrictEqual(skipped, [["SimGolf/Sounds/x.wav", "Zstandard, which the page cannot unpack"]]);
});

test("a damaged entry is skipped as damaged, an encrypted one as encrypted", async () => {
    const good = fs.readFileSync(data + "lzma-7z.zip");
    const { files } = await unzip(good);
    const sound = files["SimGolf/Sounds/Effects/Splash.wav"];
    const bad = zip([{ name: "SimGolf/Sounds/a.wav", data: sound, method: 14, raw: new Uint8Array(40).fill(7) },
        { name: "SimGolf/Sounds/b.wav", data: sound, flags: 1 }]);
    const { skipped } = await unzip(bad);
    assert.deepStrictEqual(skipped, [["SimGolf/Sounds/a.wav", "damaged (LZMA)"], ["SimGolf/Sounds/b.wav", "encrypted"]]);
});

// --- deflate without the browser's DecompressionStream (an old Safari), or with one that fails ----------------------------------
test("deflate falls back to the page's own decoder", async () => {
    const text = bytes("SimGolf ".repeat(5000) + "the end");
    const z = zip([{ name: "SimGolf/Sounds/x.wav", data: text, method: 8, raw: zlib.deflateRawSync(text) }]);
    const real = globalThis.DecompressionStream;
    try {
        for (const stand of [undefined, class { constructor() { throw new Error("not here"); } }]) {
            globalThis.DecompressionStream = stand;
            const { files, skipped, stat } = await unzip(z);
            assert.deepStrictEqual(skipped, []);
            assert.ok(Buffer.from(files["SimGolf/Sounds/x.wav"]).equals(Buffer.from(text)));
            assert.strictEqual(stat.fallback, 1);
        }
    } finally {
        globalThis.DecompressionStream = real;
    }
    const { stat } = await unzip(z);
    assert.ok(!stat.fallback, "the browser's own when it works");
});

// --- names and paths ------------------------------------------------------------------------------------------------------------
test("backslash paths", async () => {
    const z = zip([{ name: "SimGolf\\Flics\\title.fli", data: bytes("art") }, { name: "SimGolf\\Sounds\\Effects\\Splash.wav", data: bytes("RIFF") },
        { name: "SimGolf\\SimsFX\\Male\\hello.wav", data: bytes("RIFF") }]);
    assert.deepStrictEqual(await placed(z), { "flics/title.fli": "SimGolf/Flics/title.fli",
        "sounds/effects/splash.wav": "SimGolf/Sounds/Effects/Splash.wav", "simsfx/male/hello.wav": "SimGolf/SimsFX/Male/hello.wav" });
});

test("folders split by case, and an empty Sounds folder entry", async () => {
    const z = zip([{ name: "SimGolf/", data: bytes("") }, { name: "SimGolf/Sounds/", data: bytes("") }, { name: "SimGolf/SimsFX/", data: bytes("") },
        { name: "SimGolf/Flics/title.fli", data: bytes("art") }, { name: "SimGolf/Data/sims.txt", data: bytes("data") },
        { name: "SIMGOLF/SOUNDS/Effects/Splash.wav", data: bytes("RIFF") }, { name: "simgolf/sounds/Music/song.wav", data: bytes("RIFF") },
        { name: "SimGolf/simsfx/Male/hello.wav", data: bytes("RIFF") }]);
    assert.deepStrictEqual(await placed(z), { "flics/title.fli": "SimGolf/Flics/title.fli", "data/sims.txt": "SimGolf/Data/sims.txt",
        "sounds/effects/splash.wav": "SIMGOLF/SOUNDS/Effects/Splash.wav", "sounds/music/song.wav": "simgolf/sounds/Music/song.wav",
        "simsfx/male/hello.wav": "SimGolf/simsfx/Male/hello.wav" });
});

test("sounds in another copy of the folders go where the game looks, without replacing the game folder's", async () => {
    const z = zip([{ name: "SimGolf/Flics/title.fli", data: bytes("art") }, { name: "SimGolf/Interface/x.pcx", data: bytes("art") },
        { name: "SimGolf/Sounds/desktop.ini", data: bytes("ini") }, { name: "SimGolf/Sounds/Applause.wav", data: bytes("RIFF") },
        { name: "Backup/SimGolf/Sounds/Effects/Splash.wav", data: bytes("RIFF") }, { name: "Backup/SimGolf/Sounds/Applause.wav", data: bytes("RIFF") },
        { name: "SimGolf Sounds/SimsFX/Male/hello.wav", data: bytes("RIFF") }, { name: "SimGolf/Extras/sound/Music/song.mp3", data: bytes("ID3") },
        { name: "Photos/beach.wav", data: bytes("RIFF") }]);
    assert.deepStrictEqual(await placed(z), { "flics/title.fli": "SimGolf/Flics/title.fli", "interface/x.pcx": "SimGolf/Interface/x.pcx",
        "sounds/desktop.ini": "SimGolf/Sounds/desktop.ini", "sounds/applause.wav": "SimGolf/Sounds/Applause.wav",
        "extras/sound/music/song.mp3": "SimGolf/Extras/sound/Music/song.mp3", "sounds/music/song.mp3": "SimGolf/Extras/sound/Music/song.mp3",
        "sounds/effects/splash.wav": "Backup/SimGolf/Sounds/Effects/Splash.wav", "simsfx/male/hello.wav": "SimGolf Sounds/SimsFX/Male/hello.wav" });
});

test("the game folder is the one with most of the game's folders", () => {
    assert.strictEqual(f.sgRootOf(["Old/Flics/a.fli", "SimGolf/Flics/a.fli", "SimGolf/Data/x", "SimGolf/Sounds/y.wav"]), "SimGolf");
    assert.strictEqual(f.sgRootOf(["Flics/a.fli", "Sounds/a.wav"]), "");
    assert.strictEqual(f.sgRootOf(["a/flics"]), null);
});

test("names: CP437, UTF-8 flag, the Unicode path field, leading ./ and /", async () => {
    const cp = Uint8Array.from([...Buffer.from("SimGolf/Sounds/caf"), 0x82, ...Buffer.from(".wav")]);
    const utf = Buffer.from("SimGolf/Sounds/naïve.wav");
    const raw = Buffer.from("SimGolf/Sounds/x?.wav");
    const up = Buffer.from("SimGolf/Sounds/xé.wav");
    const crc = Buffer.alloc(4);
    crc.writeUInt32LE(f.sgCrc32(raw));
    const field = Buffer.concat([Buffer.from([0x75, 0x70, 5 + up.length, 0, 1]), crc, up]);
    const z = zip([{ name: cp, data: bytes("a") }, { name: utf, data: bytes("b"), flags: 0x800 }, { name: raw, data: bytes("c"), extra: field },
        { name: "./SimGolf/Flics/a.fli", data: bytes("d") }, { name: "/SimGolf//Data/b.txt", data: bytes("e") }]);
    const { files } = await unzip(z);
    assert.deepStrictEqual(Object.keys(files), ["SimGolf/Sounds/café.wav", "SimGolf/Sounds/naïve.wav", "SimGolf/Sounds/xé.wav",
        "SimGolf/Flics/a.fli", "SimGolf/Data/b.txt"]);
});

test("Zip64 records", async () => {
    const z = zip([{ name: "SimGolf/Flics/a.fli", data: bytes("art"), zip64: true }, { name: "SimGolf/Sounds/x.wav", data: bytes("RIFF....") }], true);
    const { files, skipped } = await unzip(z);
    assert.deepStrictEqual(skipped, []);
    assert.strictEqual(Buffer.from(files["SimGolf/Flics/a.fli"]).toString(), "art");
    assert.strictEqual(Buffer.from(files["SimGolf/Sounds/x.wav"]).toString(), "RIFF....");
});

// --- what the player is told ------------------------------------------------------------------------------------------------------
test("the summary counts the sounds found, used and skipped, and why", () => {
    const paths = ["game/Flics/a.fli", "game/Sounds/Music/m.wav", "game/Sounds/x.wav", "game/SimsFX/v.wav"];
    const skipped = [["S/Sounds/y.wav", "LZMA, which the page cannot unpack"], ["S/Sounds/z.wav", "LZMA, which the page cannot unpack"],
        ["S/Sounds/w.wav", "damaged (BZip2)"], ["S/Data/d.txt", "encrypted"]];
    const s = f.sgFilesSummary(paths, skipped, 7);
    assert.ok(s.text.startsWith("OpenSGolf " + f.sgVersion() + ". Sounds: found 7, used 3, skipped 3 (2 LZMA, which the page cannot unpack; " +
        "1 damaged (BZip2)). Art ✓, music: 1, Sims' voices: 1."), s.text);
    assert.ok(s.problem);
    const none = f.sgFilesSummary(["game/Flics/a.fli"], [], 0);
    assert.ok(none.problem && /found 0, used 0, skipped 0/.test(none.text) && /free OpenSGolf sounds/.test(none.text), none.text);
    const full = paths.concat(Array.from({ length: 120 }, (_, i) => "game/Sounds/Effects/s" + i + ".wav"));
    const fine = f.sgFilesSummary(full, [], 123);
    assert.ok(!fine.problem && /found 123, used 123, skipped 0\./.test(fine.text), fine.text);
    // a downloaded copy with the sounds left out but for a stray one or two points at the CD or its .iso
    const few = f.sgFilesSummary(paths, [], 3);
    assert.ok(few.problem && /Only 3 sounds in your files/.test(few.text) && /\.iso/.test(few.text), few.text);
});

test("sounds in a coding the game cannot play are counted and named", () => {
    const gsm = fs.readFileSync(data + "tone_gsm.wav"), mp2 = fs.readFileSync(data + "tone_mp2.wav");
    const ts = Buffer.from(gsm);
    ts[20] = 0x22;
    assert.strictEqual(f.sgSoundCoding(gsm), null);
    assert.strictEqual(f.sgSoundCoding(mp2), null);
    assert.strictEqual(f.sgSoundCoding(fs.readFileSync(data + "tone.ogg")), null);
    assert.strictEqual(f.sgSoundCoding(ts), "DSP Group TrueSpeech");
    assert.strictEqual(f.sgSoundCoding(Buffer.from("fLaC....")), "FLAC");
    const bad = f.sgSoundCodings([["game/Sounds/a.wav", gsm], ["game/Sounds/b.wav", ts], ["game/SimsFX/c.wav", ts], ["game/Data/d.wav", ts]]);
    assert.deepStrictEqual(bad, { n: 2, example: "DSP Group TrueSpeech" });
    const s = f.sgFilesSummary(["game/Flics/a.fli", "game/Sounds/a.wav", "game/Sounds/b.wav"], [], 2, bad);
    assert.ok(s.problem && /2 of your sounds use a format OpenSGolf can't play yet \(e\.g\. DSP Group TrueSpeech\)/.test(s.text), s.text);
});

test("a phone or tablet held upright is asked to turn; a desktop window never is", () => {
    assert.strictEqual(f.sgPortraitAsk(true, 390, 664, false), true); // iPhone 13 upright
    assert.strictEqual(f.sgPortraitAsk(true, 820, 1180, false), true); // iPad Air upright
    assert.strictEqual(f.sgPortraitAsk(true, 750, 342, false), false); // sideways
    assert.strictEqual(f.sgPortraitAsk(true, 500, 500, false), false); // square is not upright
    assert.strictEqual(f.sgPortraitAsk(true, 390, 664, true), false); // "Play anyway"
    assert.strictEqual(f.sgPortraitAsk(false, 600, 1000, false), false); // a tall desktop window
});

test("the canvas fills the view, or a 4:3 band across the middle of an upright touch screen", () => {
    assert.deepStrictEqual(f.sgCanvasRect(true, 750, 342, 0, 0), { left: 0, top: 0, width: 750, height: 342 });
    assert.deepStrictEqual(f.sgCanvasRect(false, 600, 1000, 0, 0), { left: 0, top: 0, width: 600, height: 1000 });
    assert.deepStrictEqual(f.sgCanvasRect(true, 390, 664, 0, 0), { left: 0, top: 186, width: 390, height: 293 });
    // the visual viewport's offset (Safari's bars, a pinch) carries over
    assert.deepStrictEqual(f.sgCanvasRect(true, 400, 700, 0, 10), { left: 0, top: 210, width: 400, height: 300 });
    // never taller than the view (a nearly square upright view)
    const r = f.sgCanvasRect(true, 600, 610, 0, 0);
    assert.ok(r.height <= 610 && r.top >= 0 && r.width === 600, JSON.stringify(r));
});

test("the edges a phone hides are the safe-area insets where they fall on the canvas", () => {
    // an iPhone 16 Pro Max sideways (956 x 440, the camera on the left): the whole window is the canvas
    const wide = { left: 0, top: 0, width: 956, height: 440 };
    const s = f.sgSafeInsets([62, 0, 62, 21], wide, 956, 440);
    assert.deepStrictEqual(s.map(v => Math.round(v * 1000) / 1000), [0.065, 0, 0.065, 0.048]);
    // no insets (a desktop, an iPad), or a canvas not drawn yet
    assert.deepStrictEqual(f.sgSafeInsets([0, 0, 0, 0], wide, 956, 440), [0, 0, 0, 0]);
    assert.deepStrictEqual(f.sgSafeInsets([62, 0, 62, 21], { left: 0, top: 0, width: 0, height: 0 }, 956, 440), [0, 0, 0, 0]);
    // upright, "Play anyway": the 4:3 band across the middle is clear of the cut-out at the top and the home bar
    const band = f.sgCanvasRect(true, 440, 956, 0, 0);
    assert.deepStrictEqual(f.sgSafeInsets([0, 62, 0, 34], band, 440, 956), [0, 0, 0, 0]);
    // a canvas that stops short of the edge takes only what is left of the inset
    assert.deepStrictEqual(f.sgSafeInsets([40, 0, 0, 0], { left: 30, top: 0, width: 100, height: 100 }, 130, 100), [0.1, 0, 0, 0]);
});

test("the start page's dark palette is the same for the device's dark scheme and the theme button's Dark", () => {
    const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
    const block = re => {
        const m = html.match(re);
        assert.ok(m, "no block for " + re);
        return m[1].replace(/\s+/g, " ").trim();
    };
    const device = block(/@media \(prefers-color-scheme: dark\) \{\s*:root:not\(\[data-theme="light"\]\) \{([^}]*)\}\s*\}/);
    const button = block(/\n\s*:root\[data-theme="dark"\] \{([^}]*)\}/);
    assert.ok(device.includes("--panel:"), device);
    assert.strictEqual(button, device);
    // and no other dark-only rules that the button's Dark would miss
    assert.strictEqual(html.match(/prefers-color-scheme/g).length, 1);
});

test("the sound context sleeps while the game has its sound asleep, and only the game wakes it", () => {
    const act = f.sgSoundAction;
    // asleep (the Pause in the background extra, the window in the background): a running context is suspended, in a
    // hidden page and in a shown one that lost the focus
    assert.strictEqual(act("running", true, true), "suspend");
    assert.strictEqual(act("running", true, false), "suspend");
    // and stays suspended, the page shown again or tapped, until the game wakes it
    assert.strictEqual(act("suspended", true, false), "");
    assert.strictEqual(act("interrupted", true, false), "");
    // awake: resumed while shown, as before
    assert.strictEqual(act("suspended", false, false), "resume");
    assert.strictEqual(act("interrupted", false, false), "resume");
    assert.strictEqual(act("suspended", false, true), "");
    assert.strictEqual(act("running", false, false), "");
    // without the extra a page in the background plays on
    assert.strictEqual(act("running", false, true), "");
    // no context made yet, or one closed (sgStartAudio makes another)
    for (const asleep of [false, true]) {
        for (const state of [undefined, null, "closed"]) assert.strictEqual(act(state, asleep, false), "", state + " " + asleep);
    }
});

test("every function the game asks of the page is one the page gives it", () => {
    // (gl.js links a function the page does not give to a stand-in that does nothing, so a misspelt one fails silently)
    const crates = path.join(__dirname, "../crates");
    const asked = new Set();
    for (const file of fs.readdirSync(crates, { recursive: true })) {
        if (!file.endsWith(".rs")) continue;
        const src = fs.readFileSync(path.join(crates, file), "utf8");
        for (const m of src.matchAll(/^\s*(?:pub\s+)?fn\s+(sg_\w+)\s*\([^)]*\)\s*(?:->\s*[\w:]+\s*)?;/gm)) asked.add(m[1]);
    }
    assert.ok(asked.has("sg_sound_sleep") && asked.has("sg_fs_write"), [...asked].join(" "));
    const page = fs.readFileSync(path.join(__dirname, "opensgolf.js"), "utf8");
    for (const name of asked) assert.ok(page.includes("env." + name + " = function"), name + " is not given by web/opensgolf.js");
});

(async () => {
    for (const [name, fn] of tests) {
        try {
            await fn();
        } catch (e) {
            failed++;
            console.error("FAIL " + name + ": " + (e && e.stack ? e.stack : e));
        }
    }
    console.log("web/test.js: " + (tests.length - failed) + " of " + tests.length + " passed");
    process.exit(failed ? 1 : 0);
})();
