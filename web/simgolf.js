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
        env.sg_keyboard = function (show) { sgKeyboard(show !== 0); };
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
        env.sg_fs_remove = function (ptr, len) {
            const path = sgStr(ptr, len);
            const k = sgKey(path);
            if (!sgfs.files.delete(k)) return 0;
            const parts = path.replace(/\\/g, "/").split("/").filter(s => s.length > 0);
            const kids = sgfs.dirs.get(sgKey(parts.slice(0, -1).join("/")));
            if (kids) kids.delete(parts[parts.length - 1].toLowerCase());
            try {
                for (let i = localStorage.length - 1; i >= 0; i--) {
                    const lk = localStorage.key(i);
                    if (lk && lk.startsWith(SAVE_PREFIX) && sgKey(lk.substring(SAVE_PREFIX.length)) === k) localStorage.removeItem(lk);
                }
            } catch (e) {
                console.warn("could not remove " + path + " from local storage", e);
            }
            return 1;
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
        const r = indexedDB.open("simgolf", 2);
        r.onupgradeneeded = () => {
            // "files": the game folder; "hd": the player's own HD art pack (optional, docs/HD.md)
            for (const s of ["files", "hd"]) if (!r.result.objectStoreNames.contains(s)) r.result.createObjectStore(s);
        };
        r.onsuccess = () => ok(r.result);
        r.onerror = () => fail(r.error);
    });
}
async function sgStoreAll(entries, store = "files") {
    try {
        const db = await sgDb();
        await new Promise((ok, fail) => {
            const tx = db.transaction(store, "readwrite");
            const st = tx.objectStore(store);
            st.clear();
            for (const [path, bytes] of entries) st.put(bytes, path);
            tx.oncomplete = ok;
            tx.onerror = () => fail(tx.error);
        });
    } catch (e) {
        console.warn("could not keep the game files in this browser", e);
    }
}
async function sgLoadStored(store = "files") {
    try {
        const db = await sgDb();
        return await new Promise((ok, fail) => {
            const out = [];
            const tx = db.transaction(store, "readonly");
            const req = tx.objectStore(store).openCursor();
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

// --- the optional HD art pack -------------------------------------------------------------------------------------------------
// A pack the player made with tools/hd_pack from their own copy of the game (never downloaded from anywhere). Its files go
// under "HD/", where the game looks for it (the save folder's HD folder), and are kept in this browser like the game files.
function sgHdStatus(t) {
    document.getElementById("hdstatus").textContent = t;
}
function sgHdRootOf(paths) {
    let best = null;
    for (const p of paths) {
        const parts = p.split("/");
        if (parts[parts.length - 1].toLowerCase() === "manifest.json") {
            const root = parts.slice(0, -1).join("/");
            if (best === null || root.length < best.length) best = root;
        }
    }
    return best;
}
async function sgUseHd(entries) {
    const root = sgHdRootOf(entries.map(e => e[0]));
    if (root === null) {
        sgHdStatus("That is not an HD pack (no manifest.json found). Pick the folder tools/hd_pack wrote, or a .zip of it.");
        return;
    }
    const keep = [];
    for (const [path, bytes] of entries) {
        if (!path.startsWith(root)) continue;
        const low = path.toLowerCase();
        if (!low.endsWith(".png") && !low.endsWith(".json")) continue;
        const rel = "HD/" + path.substring(root.length).replace(/^\//, "");
        sgAddFile(rel, bytes);
        keep.push([rel, bytes]);
    }
    // start in HD from now on (the game's Preferences switch it back to Classic)
    try {
        const k = SAVE_PREFIX + "settings.json";
        let s = {};
        try { s = JSON.parse(atob(localStorage.getItem(k) || "")) || {}; } catch (e) { s = {}; }
        s.graphics = "hd";
        localStorage.setItem(k, btoa(JSON.stringify(s)));
    } catch (e) {}
    sgHdStatus("HD pack ready (" + keep.length + " files). The game starts in HD; Preferences switch between HD and Classic.");
    if (document.getElementById("remember").checked) sgStoreAll(keep, "hd");
}
async function sgLoadHdFolder(fileList) {
    const entries = [];
    const files = Array.from(fileList);
    for (let i = 0; i < files.length; i++) {
        entries.push([files[i].webkitRelativePath, new Uint8Array(await files[i].arrayBuffer())]);
        if (i % 100 === 0) {
            sgHdStatus("Reading the HD pack: " + (i + 1) + " of " + files.length);
            await new Promise(r => setTimeout(r, 0));
        }
    }
    await sgUseHd(entries);
}
async function sgLoadHdZip(file) {
    if (!file) return;
    const entries = [];
    try {
        await sgUnzip(await file.arrayBuffer(), async (name, bytes, i, n) => {
            entries.push([name, bytes]);
            if (i % 100 === 0) {
                sgHdStatus("Unpacking the HD pack: " + (i + 1) + " of " + n);
                await new Promise(r => setTimeout(r, 0));
            }
        });
    } catch (e) {
        sgHdStatus("Could not read that zip: " + e.message);
        return;
    }
    await sgUseHd(entries);
}
async function sgForgetHd() {
    try {
        const db = await sgDb();
        db.transaction("hd", "readwrite").objectStore("hd").clear();
    } catch (e) {}
    sgHdStatus("The HD pack was removed from this browser.");
}

async function sgInit() {
    const hd = await sgLoadStored("hd");
    if (hd.length > 0) {
        for (const [path, bytes] of hd) sgAddFile(path, bytes);
        sgHdStatus("Your HD pack is kept in this browser (" + hd.length + " files).");
    }
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

// Typing on a phone: the game asks for the keyboard (sg_keyboard) while a name box is open. A hidden text box takes the
// focus so the phone shows its keyboard, and what is typed goes to the game as key presses. The box keeps one space in it
// so a backspace always has something to delete and can be seen.
let sgKbd = null;
function sgKeyTap(code) {
    wasm_exports.key_down(code, 0, false);
    wasm_exports.key_up(code, 0);
}
function sgKeyboard(show) {
    if (!sgKbd) {
        sgKbd = document.createElement("input");
        sgKbd.type = "text";
        sgKbd.autocomplete = "off";
        sgKbd.setAttribute("autocapitalize", "off");
        sgKbd.setAttribute("autocorrect", "off");
        sgKbd.spellcheck = false;
        // 16px stops iOS from zooming the page when the box takes the focus
        sgKbd.style.cssText = "position:fixed;left:0;bottom:0;width:1px;height:1px;opacity:0;border:0;padding:0;font-size:16px";
        document.body.appendChild(sgKbd);
        sgKbd.addEventListener("input", () => {
            const v = sgKbd.value;
            if (v.length < 1) sgKeyTap(259); // backspace
            else for (const ch of v.substring(1)) wasm_exports.key_press(ch.codePointAt(0));
            sgKbd.value = " ";
        });
        sgKbd.addEventListener("keydown", e => {
            if (e.key === "Enter") { sgKeyTap(257); e.preventDefault(); }
            else if (e.key === "Escape") { sgKeyTap(256); e.preventDefault(); }
        });
    }
    if (show) {
        sgKbd.value = " ";
        sgKbd.focus();
    } else {
        sgKbd.blur();
        document.getElementById("glcanvas").focus();
    }
}
