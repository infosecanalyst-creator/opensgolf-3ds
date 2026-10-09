// SimGolf in the browser: the page side of the game's file access (crates/sg-core/src/fsutil.rs).
// The player picks the folder of their own SimGolf copy; its files are read into memory here (nothing is uploaded anywhere)
// and the game reads them through the sg_fs_* functions below. Lookups are case-insensitive, like Windows.
"use strict";

const sgfs = { files: new Map(), dirs: new Map(), list: [] };
const enc = new TextEncoder();
const dec = new TextDecoder();

function sgKey(p) {
    return p.replace(/\\/g, "/").replace(/\/+/g, "/").replace(/\/$/, "").replace(/^\.\//, "").toLowerCase();
}

function sgAddFile(path, bytes) {
    const parts = path.replace(/\\/g, "/").split("/").filter(s => s.length > 0);
    sgfs.files.set(sgKey(parts.join("/")), bytes);
    for (let i = 0; i < parts.length; i++) {
        const dir = sgKey(parts.slice(0, i).join("/"));
        let kids = sgfs.dirs.get(dir);
        if (!kids) {
            kids = new Map();
            sgfs.dirs.set(dir, kids);
        }
        kids.set(parts[i].toLowerCase(), parts[i]);
    }
}

function sgStr(ptr, len) {
    return dec.decode(new Uint8Array(wasm_memory.buffer, ptr, len));
}

// Files the game writes (saved courses) are kept in the browser's local storage.
const SAVE_PREFIX = "simgolf-file:";
function sgRestoreSaves() {
    for (let i = 0; i < localStorage.length; i++) {
        const k = localStorage.key(i);
        if (k && k.startsWith(SAVE_PREFIX)) {
            const bin = atob(localStorage.getItem(k));
            const bytes = new Uint8Array(bin.length);
            for (let j = 0; j < bin.length; j++) bytes[j] = bin.charCodeAt(j);
            sgAddFile(k.substring(SAVE_PREFIX.length), bytes);
        }
    }
}

miniquad_add_plugin({
    name: "simgolf_fs",
    version: 1,
    register_plugin: function (importObject) {
        const env = importObject.env;
        env.sg_fs_size = function (ptr, len) {
            const f = sgfs.files.get(sgKey(sgStr(ptr, len)));
            return f ? f.length : -1;
        };
        env.sg_fs_read = function (ptr, len, dst) {
            const f = sgfs.files.get(sgKey(sgStr(ptr, len)));
            if (f) new Uint8Array(wasm_memory.buffer, dst, f.length).set(f);
        };
        env.sg_fs_write = function (ptr, len, data, dataLen) {
            const path = sgStr(ptr, len);
            const bytes = new Uint8Array(wasm_memory.buffer, data, dataLen).slice();
            sgAddFile(path, bytes);
            try {
                let bin = "";
                for (let i = 0; i < bytes.length; i++) bin += String.fromCharCode(bytes[i]);
                localStorage.setItem(SAVE_PREFIX + path, btoa(bin));
            } catch (e) {
                console.warn("could not keep " + path + " in local storage", e);
            }
        };
        env.sg_fs_kind = function (ptr, len) {
            const k = sgKey(sgStr(ptr, len));
            if (sgfs.files.has(k)) return 1;
            if (sgfs.dirs.has(k)) return 2;
            return 0;
        };
        env.sg_fs_list = function (ptr, len) {
            const kids = sgfs.dirs.get(sgKey(sgStr(ptr, len)));
            sgfs.list = kids ? Array.from(kids.values()) : [];
            return sgfs.list.length;
        };
        env.sg_fs_list_item = function (i, dst, cap) {
            const b = enc.encode(sgfs.list[i] || "");
            const n = Math.min(b.length, cap);
            new Uint8Array(wasm_memory.buffer, dst, n).set(b.subarray(0, n));
            return n;
        };
    },
});

// Loading the player's files. The game folder is the one that holds Flics, Sounds and Data; the player may pick it, any folder
// above it, or a .zip of it. Videos (.bik) and Windows binaries are not needed and are skipped. The files are kept in this
// browser (IndexedDB) so the next visit starts at once; "Forget my game files" removes them.
function sgSkip(low) {
    return low.endsWith(".bik") || low.endsWith(".exe") || low.endsWith(".dll") || low.endsWith(".lib") || low.endsWith("/");
}

function sgRootOf(paths) {
    for (const p of paths) {
        const parts = p.split("/");
        const i = parts.findIndex(x => x.toLowerCase() === "flics");
        if (i >= 0) return parts.slice(0, i).join("/");
    }
    return null;
}

function sgStatus(t) {
    document.getElementById("status").textContent = t;
}

// --- browser storage of the game files --------------------------------------------------------------------------------------
function sgDb() {
    return new Promise((ok, fail) => {
        const r = indexedDB.open("simgolf", 1);
        r.onupgradeneeded = () => r.result.createObjectStore("files");
        r.onsuccess = () => ok(r.result);
        r.onerror = () => fail(r.error);
    });
}
async function sgStoreAll(entries) {
    try {
        const db = await sgDb();
        await new Promise((ok, fail) => {
            const tx = db.transaction("files", "readwrite");
            const st = tx.objectStore("files");
            st.clear();
            for (const [path, bytes] of entries) st.put(bytes, path);
            tx.oncomplete = ok;
            tx.onerror = () => fail(tx.error);
        });
    } catch (e) {
        console.warn("could not keep the game files in this browser", e);
    }
}
async function sgLoadStored() {
    try {
        const db = await sgDb();
        return await new Promise((ok, fail) => {
            const out = [];
            const tx = db.transaction("files", "readonly");
            const req = tx.objectStore("files").openCursor();
            req.onsuccess = () => {
                const c = req.result;
                if (c) { out.push([c.key, c.value]); c.continue(); } else ok(out);
            };
            req.onerror = () => fail(req.error);
        });
    } catch (e) {
        return [];
    }
}
async function sgForget() {
    try {
        const db = await sgDb();
        db.transaction("files", "readwrite").objectStore("files").clear();
    } catch (e) {}
    sgStatus("Your game files were removed from this browser.");
    document.getElementById("again").style.display = "none";
}

// --- reading a .zip (stored or deflated entries) ----------------------------------------------------------------------------
async function sgUnzip(buf, onEntry) {
    const v = new DataView(buf);
    let e = buf.byteLength - 22;
    while (e >= 0 && v.getUint32(e, true) !== 0x06054b50) e--;
    if (e < 0) throw new Error("not a zip file");
    const count = v.getUint16(e + 10, true);
    let p = v.getUint32(e + 16, true);
    const dec8 = new TextDecoder();
    for (let i = 0; i < count; i++) {
        const method = v.getUint16(p + 10, true);
        const csize = v.getUint32(p + 20, true);
        const nlen = v.getUint16(p + 28, true), xlen = v.getUint16(p + 30, true), clen = v.getUint16(p + 32, true);
        const local = v.getUint32(p + 42, true);
        const name = dec8.decode(new Uint8Array(buf, p + 46, nlen));
        p += 46 + nlen + xlen + clen;
        if (sgSkip(name.toLowerCase())) continue;
        const lnlen = v.getUint16(local + 26, true), lxlen = v.getUint16(local + 28, true);
        const data = new Uint8Array(buf, local + 30 + lnlen + lxlen, csize);
        let bytes;
        if (method === 0) bytes = data.slice();
        else if (method === 8) bytes = new Uint8Array(await new Response(new Blob([data]).stream().pipeThrough(new DecompressionStream("deflate-raw"))).arrayBuffer());
        else continue;
        await onEntry(name, bytes, i, count);
    }
}

// --- starting --------------------------------------------------------------------------------------------------------------
function sgStart() {
    sgRestoreSaves();
    document.getElementById("picker").style.display = "none";
    const canvas = document.getElementById("glcanvas");
    canvas.style.display = "block";
    canvas.focus();
    load("simgolf.wasm");
}

async function sgUse(entries) {
    const root = sgRootOf(entries.map(e => e[0]));
    if (root === null) {
        sgStatus("That does not look like a SimGolf game folder (no Flics folder found). Pick the folder that contains Flics, Sounds and Data, or a .zip of it.");
        return;
    }
    const keep = [];
    for (const [path, bytes] of entries) {
        if (!path.startsWith(root)) continue;
        const rel = "game/" + path.substring(root.length).replace(/^\//, "");
        sgAddFile(rel, bytes);
        keep.push([rel, bytes]);
    }
    // play at once; the copy kept for next time is written in the background (it can take minutes for a whole game folder)
    const remember = document.getElementById("remember").checked;
    sgStart();
    if (remember) sgStoreAll(keep);
}

async function sgLoadFolder(fileList) {
    const files = Array.from(fileList).filter(f => !sgSkip(f.webkitRelativePath.toLowerCase()));
    if (sgRootOf(files.map(f => f.webkitRelativePath)) === null) {
        sgStatus("That folder does not look like a SimGolf game folder (no Flics folder found). Pick the folder that contains Flics, Sounds and Data.");
        return;
    }
    const entries = [];
    let bytes = 0;
    for (let i = 0; i < files.length; i++) {
        entries.push([files[i].webkitRelativePath, new Uint8Array(await files[i].arrayBuffer())]);
        bytes += files[i].size;
        if (i % 50 === 0 || i === files.length - 1) {
            sgStatus("Reading your game files: " + (i + 1) + " of " + files.length + " (" + Math.round(bytes / 1048576) + " MB)");
            await new Promise(r => setTimeout(r, 0));
        }
    }
    await sgUse(entries);
}

async function sgLoadZip(file) {
    if (!file) return;
    sgStatus("Opening " + file.name + "...");
    const entries = [];
    try {
        await sgUnzip(await file.arrayBuffer(), async (name, bytes, i, n) => {
            entries.push([name, bytes]);
            if (i % 50 === 0) {
                sgStatus("Unpacking your game files: " + (i + 1) + " of " + n);
                await new Promise(r => setTimeout(r, 0));
            }
        });
    } catch (e) {
        sgStatus("Could not read that zip: " + e.message);
        return;
    }
    await sgUse(entries);
}

async function sgInit() {
    const stored = await sgLoadStored();
    if (stored.length > 0) {
        document.getElementById("again").style.display = "block";
        document.getElementById("playstored").onclick = () => {
            sgStatus("Starting...");
            for (const [path, bytes] of stored) sgAddFile(path, bytes);
            sgStart();
        };
    }
}

// Sound: the game mixes at 44.1 kHz; Web Audio pulls blocks from it through sg_audio_fill. Browsers only allow sound after
// the player interacts with the page, so the output starts on the first click or key.
let sgAudio = null;
function sgStartAudio() {
    if (sgAudio || typeof wasm_exports === "undefined" || !wasm_exports.sg_audio_fill) return;
    const ctx = new (window.AudioContext || window.webkitAudioContext)();
    const node = ctx.createScriptProcessor(2048, 0, 2);
    node.onaudioprocess = e => {
        const n = e.outputBuffer.length;
        const ptr = wasm_exports.sg_audio_fill(ctx.sampleRate, n);
        const l = e.outputBuffer.getChannelData(0), r = e.outputBuffer.getChannelData(1);
        if (!ptr) { l.fill(0); r.fill(0); return; }
        const src = new Float32Array(wasm_memory.buffer, ptr, n * 2);
        for (let i = 0; i < n; i++) { l[i] = src[2 * i]; r[i] = src[2 * i + 1]; }
    };
    node.connect(ctx.destination);
    sgAudio = ctx;
}
// browsers only allow sound after the player's first gesture: a click, a key or a touch
window.addEventListener("mousedown", sgStartAudio);
window.addEventListener("keydown", sgStartAudio);
window.addEventListener("touchstart", sgStartAudio, { passive: true });
window.addEventListener("pointerdown", sgStartAudio);
