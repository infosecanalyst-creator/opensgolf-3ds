// SimGolf in the browser: the page side of the game's file access (crates/sg-core/src/fsutil.rs).
// The player picks the folder of their own SimGolf copy; its files are read into memory here (nothing is uploaded anywhere)
// and the game reads them through the sg_fs_* functions below. Lookups are case-insensitive, like Windows.
"use strict";

// miniquad's depth textures ask for the unsized DEPTH_COMPONENT format (WebGL 1's), which WebGL 2 refuses, leaving the
// render target incomplete. Photo mode (crates/simgolf/src/photo.rs) draws the course offscreen with a depth buffer, so
// such a texture is given the sized format of its type instead (16 bits for UNSIGNED_SHORT, as miniquad asks).
(function () {
    if (typeof WebGL2RenderingContext === "undefined") return;
    const proto = WebGL2RenderingContext.prototype;
    const texImage2D = proto.texImage2D;
    proto.texImage2D = function (target, level, internal, ...rest) {
        // rest: width, height, border, format, type, pixels
        if (internal === 0x1902 && rest.length >= 6 && rest[3] === 0x1902) {
            internal = rest[4] === 0x1405 ? 0x81a6 : rest[4] === 0x1406 ? 0x8cac : 0x81a5;
        }
        return texImage2D.call(this, target, level, internal, ...rest);
    };
})();

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

// The frame rate cap (framerate.rs). A browser draws on the display's refresh (v-sync is always on); with a cap the page
// skips animation frames until the next frame is due, so gl.js calls the game less often. The game's simulation keeps its
// own 87 ms ticks whatever the frame rate.
let sgFrameCap = 0;
let sgFrameDue = 0;
const sgRequestFrame = window.requestAnimationFrame.bind(window);
window.requestAnimationFrame = function (callback) {
    const tick = t => {
        if (sgFrameCap > 0) {
            const period = 1000 / sgFrameCap;
            // a millisecond early still counts, so a 60 cap on a 60 Hz display skips nothing
            if (t < sgFrameDue - 1) return sgRequestFrame(tick);
            sgFrameDue = t - sgFrameDue < period ? sgFrameDue + period : t + period;
        }
        callback(t);
    };
    return sgRequestFrame(tick);
};

miniquad_add_plugin({
    name: "simgolf_fs",
    version: 1,
    register_plugin: function (importObject) {
        const env = importObject.env;
        env.sg_keyboard = function (show) { sgKeyboard(show !== 0); };
        // the frame rate cap (Enhanced graphics > Display; framerate.rs): frames a second, 0 for none
        env.sg_frame_cap = function (fps) { sgFrameCap = fps; };
        // the game's quit (the title menu's exit button): a tab cannot close itself, so back to the start page
        env.sg_quit = function () { location.reload(); };
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
            // a picture the player took (Shift+F6 screenshot, a SimFoto, F10 on a great shot, a photo mode photo) is handed to the browser as a
            // download: it would be out of reach (and soon over the quota) in local storage. The accomplishment snapshots
            // the board shows (snapshots/accomp<n>.png) are kept like the saves.
            const low = sgKey(path);
            if (low.endsWith(".png") && !/(^|\/)accomp\d+\.png$/.test(low)) {
                sgDownload(path.split(/[\\/]/).pop(), bytes);
                return;
            }
            sgAddFile(path, bytes);
            // a pack the game downloaded (Preferences > Content > Find more) or exported goes with the packs the player added
            if (sgKey(path).startsWith("packs/")) {
                sgStoreAdd([[path, bytes]], "packs");
                return;
            }
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
            if (k.startsWith("packs/")) {
                // an older version of a downloaded pack
                sgDb().then(db => db.transaction("packs", "readwrite").objectStore("packs").delete(path)).catch(() => {});
            }
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
        // Downloads for Preferences > Content > Find more (crates/simgolf/src/net.rs): the game starts a fetch and polls it
        // each frame. The content index and the packs must be served with CORS allowed (GitHub's raw files and releases are).
        env.sg_net_fetch = function (ptr, len) {
            const url = sgStr(ptr, len);
            const id = sgnet.next++;
            const req = { status: -2, data: null, error: "" };
            sgnet.reqs.set(id, req);
            fetch(url)
                .then(r => {
                    if (!r.ok) throw new Error(url + ": HTTP " + r.status);
                    return r.arrayBuffer();
                })
                .then(b => {
                    req.data = new Uint8Array(b);
                    req.status = req.data.length;
                })
                .catch(e => {
                    req.error = String(e && e.message ? e.message : e);
                    req.status = -1;
                });
            return id;
        };
        env.sg_net_status = function (id) {
            const r = sgnet.reqs.get(id);
            return r ? r.status : -1;
        };
        env.sg_net_take = function (id, dst) {
            const r = sgnet.reqs.get(id);
            if (r && r.data) new Uint8Array(wasm_memory.buffer, dst, r.data.length).set(r.data);
            sgnet.reqs.delete(id);
        };
        env.sg_net_error = function (id, dst, cap) {
            const r = sgnet.reqs.get(id);
            const b = enc.encode(r ? r.error : "no such download");
            const n = Math.min(b.length, cap);
            new Uint8Array(wasm_memory.buffer, dst, n).set(b.subarray(0, n));
            sgnet.reqs.delete(id);
            return n;
        };
    },
});

const sgnet = { next: 1, reqs: new Map() };

// Hands a file to the browser as a download.
function sgDownload(name, bytes) {
    const url = URL.createObjectURL(new Blob([bytes], { type: name.toLowerCase().endsWith(".png") ? "image/png" : "application/octet-stream" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = name;
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 10000);
}

// Loading the player's files. The game folder is the one that holds Flics, Sounds and Data; the player may pick it, any folder
// above it, or a .zip of it. Videos (.bik) and Windows binaries are not needed and are skipped. The files are kept in this
// browser (IndexedDB) so the next visit starts at once; "Forget my game files" removes them.
function sgSkip(low) {
    // a zip made on a Mac carries the files' resource forks in __MACOSX/ and as "._" files: not game files
    if (low.startsWith("__macosx/") || low.includes("/__macosx/") || low.startsWith("._") || low.includes("/._")) return true;
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
        const r = indexedDB.open("simgolf", 3);
        r.onupgradeneeded = () => {
            // "files": the game folder; "hd": the player's own HD art pack (optional, docs/HD.md); "packs": content packs
            // the player added (docs/MODDING.md)
            for (const s of ["files", "hd", "packs"]) if (!r.result.objectStoreNames.contains(s)) r.result.createObjectStore(s);
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
// Adds entries to a store without clearing it (content packs are added one at a time).
async function sgStoreAdd(entries, store) {
    try {
        const db = await sgDb();
        await new Promise((ok, fail) => {
            const tx = db.transaction(store, "readwrite");
            const st = tx.objectStore(store);
            for (const [path, bytes] of entries) st.put(bytes, path);
            tx.oncomplete = ok;
            tx.onerror = () => fail(tx.error);
        });
    } catch (e) {
        console.warn("could not keep the pack in this browser", e);
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
    let unread = 0;
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
        else {
            // Deflate64, BZIP2, LZMA... (7-Zip's and Windows' stronger settings): the browser cannot unpack them
            if (unread++ < 5) console.warn("zip: cannot unpack " + name + " (compression method " + method + ")");
            continue;
        }
        await onEntry(name, bytes, i, count);
    }
    return unread;
}

// --- starting --------------------------------------------------------------------------------------------------------------
function sgStart() {
    sgRestoreSaves();
    document.getElementById("picker").style.display = "none";
    const canvas = document.getElementById("glcanvas");
    // a right-click (or a Mac's Ctrl+click) is the game's: no browser menu over the canvas, which would also take the
    // button's release
    canvas.addEventListener("contextmenu", e => e.preventDefault());
    canvas.style.display = "block";
    canvas.focus();
    // the sound output (see sgStartAudio)
    sgSnd.gameOn = true;
    sgStartAudio();
    load("opensgolf.wasm");
}

// No game folder: the game finds no original files under game/ and plays on the free base set (opensgolf-free), which it
// draws itself.
function sgPlayFree() {
    sgStatus("Starting with the free OpenSGolf graphics...");
    sgStart();
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
    let unread = 0;
    try {
        unread = await sgUnzip(await file.arrayBuffer(), async (name, bytes, i, n) => {
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
    if (unread > 0) {
        // the game still starts (missing sounds are replaced by the free set's), but the player should know why
        alert(unread + " files in that zip use a compression this browser cannot unpack (they were skipped). " +
            "If something is missing, zip the folder again with normal (Deflate) compression, or pick the folder itself.");
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

// --- content packs (mods) -----------------------------------------------------------------------------------------------------
// Packs the player adds (docs/MODDING.md) go under "packs/", the browser version's packs folder: a folder pack as its files
// (packs/<folder>/pack.toml ...), a .zip as the zip itself (packs/<name>.zip; the game reads zips). They are kept in this
// browser. The game lists them in System Functions > Preferences > Content packs, where they are switched on.
function sgPackStatus(t) {
    document.getElementById("packstatus").textContent = t;
}
function sgPackName(n) {
    return n.replace(/[\\/:*?"<>|]/g, "_");
}
async function sgLoadPackFolder(fileList) {
    const files = Array.from(fileList);
    const tomls = files.map(f => f.webkitRelativePath).filter(p => p.toLowerCase().endsWith("/pack.toml"));
    if (tomls.length === 0) {
        sgPackStatus("That folder is not a content pack (no pack.toml found).");
        return;
    }
    // the shallowest pack.toml marks the pack's root
    const rootPath = tomls.sort((a, b) => a.split("/").length - b.split("/").length)[0];
    const root = rootPath.substring(0, rootPath.length - "pack.toml".length);
    const name = sgPackName(root.replace(/\/$/, "").split("/").pop() || "pack");
    const keep = [];
    for (const f of files) {
        const p = f.webkitRelativePath;
        if (!p.startsWith(root)) continue;
        const rel = "packs/" + name + "/" + p.substring(root.length);
        const bytes = new Uint8Array(await f.arrayBuffer());
        sgAddFile(rel, bytes);
        keep.push([rel, bytes]);
    }
    if (document.getElementById("remember").checked) await sgStoreAdd(keep, "packs");
    sgPackStatus("Added " + name + " (" + keep.length + " files). Switch it on in the game: System Functions, Preferences, Content packs.");
}
async function sgLoadPackZip(file) {
    if (!file) return;
    const bytes = new Uint8Array(await file.arrayBuffer());
    if (bytes.length < 4 || bytes[0] !== 0x50 || bytes[1] !== 0x4b) {
        sgPackStatus("That is not a .zip file.");
        return;
    }
    const rel = "packs/" + sgPackName(file.name);
    sgAddFile(rel, bytes);
    if (document.getElementById("remember").checked) await sgStoreAdd([[rel, bytes]], "packs");
    sgPackStatus("Added " + file.name + ". Switch it on in the game: System Functions, Preferences, Content packs.");
}
async function sgForgetPacks() {
    try {
        const db = await sgDb();
        db.transaction("packs", "readwrite").objectStore("packs").clear();
    } catch (e) {}
    for (const k of Array.from(sgfs.files.keys())) if (k.startsWith("packs/")) sgfs.files.delete(k);
    sgfs.dirs.delete("packs");
    sgPackStatus("The content packs were removed from this browser.");
}

async function sgInit() {
    const packs = await sgLoadStored("packs");
    if (packs.length > 0) {
        for (const [path, bytes] of packs) sgAddFile(path, bytes);
        const names = new Set(packs.map(([p]) => p.split("/")[1]));
        sgPackStatus("Content packs kept in this browser: " + Array.from(names).join(", ") + ".");
    }
    const hd = await sgLoadStored("hd");
    if (hd.length > 0) {
        for (const [path, bytes] of hd) sgAddFile(path, bytes);
        sgHdStatus("Your HD pack is kept in this browser (" + hd.length + " files).");
    }
    const stored = await sgLoadStored();
    if (stored.length > 0) {
        document.getElementById("again").style.display = "";
        document.getElementById("playstored").onclick = () => {
            sgStatus("Starting...");
            for (const [path, bytes] of stored) sgAddFile(path, bytes);
            sgStart();
        };
    }
}

// Sound. The game mixes at 44.1 kHz (sg_core::mixer) and hands out blocks through sg_audio_fill. The page mixes a little
// ahead (SG_LEAD seconds) from a timer and queues the blocks in an AudioWorklet, which plays them on the browser's audio
// thread: a long frame of the game (loading a theme, opening a report, a first sound read from the player's files) no longer
// stalls the output. A ScriptProcessorNode pulled its blocks on the page's own thread, and any frame over about 70 ms came
// out as repeated and skipped stretches (Chrome) or gaps (Firefox): the crackle and stutter some players heard.
// The context runs at the mixer's 44.1 kHz where the browser allows it, so the browser's own resampler (not a linear one)
// meets the device's rate (48 kHz, or 16/24 kHz on a Bluetooth headset).
// Browsers only allow sound after the player's first gesture. The context is made when the game starts (sgStart: inside the
// click on "Play with free graphics" or "Continue playing"; after the player's folder or zip has been read, the page already
// counts a gesture) and resumed on every later click, key or touch. It is not made on the start page: a context made there
// by the click that opened the file picker sat silent through the picker and the reading of the files (a minute or more),
// and in Edge on Windows the game's sound then never came out with the player's own files while free graphics played.
// A watchdog (sgAudioWatch) also starts the output again if it stops consuming the sound, and says so in the console.
const SG_LEAD = 0.2, SG_LEAD_MAX = 0.6, SG_LEAD_HIDDEN = 1.2;
const SG_WORKLET = `
class SgOut extends AudioWorkletProcessor {
    constructor() {
        super();
        this.q = []; this.off = 0; this.left = 0; this.recv = 0; this.fed = false; this.dry = false; this.done = 0;
        this.port.onmessage = e => { this.q.push(e.data); this.left += e.data.length >> 1; this.recv += e.data.length >> 1; this.fed = true; };
    }
    process(inputs, outputs) {
        const l = outputs[0][0], r = outputs[0][1] || l, n = l.length;
        let i = 0;
        while (i < n && this.q.length) {
            const b = this.q[0], frames = b.length >> 1, k = Math.min(n - i, frames - this.off);
            for (let j = 0, s = this.off * 2; j < k; j++, s += 2) { l[i + j] = b[s]; r[i + j] = b[s + 1]; }
            i += k; this.off += k; this.done += k; this.left -= k;
            if (this.off === frames) { this.q.shift(); this.off = 0; }
        }
        let under = 0;
        if (i < n) {
            l.fill(0, i); r.fill(0, i);
            if (this.fed && !this.dry) under = 1;
            this.dry = true;
        } else this.dry = false;
        // what is left to play as of the end of this block, for the page to reckon from the audio clock
        if (this.done >= 1024 || under) { this.port.postMessage([this.recv, this.left, currentFrame + n, under]); this.done = 0; }
        return true;
    }
}
registerProcessor("sg-out", SgOut);
`;
const sgSnd = { ctx: null, node: null, rate: 44100, posted: 0, rep: null, lead: SG_LEAD, calm: 0, under: 0, failed: false, shown: 0, since: 0, note: null,
    gameOn: false, repAt: 0, postAt: 0, clock: -1, clockAt: 0, tick: 0, restarts: 0 };

function sgStartAudio() {
    const s = sgSnd;
    if (!s.gameOn) return; // made when the game starts (sgStart), see above
    // a context made before the browser counted a gesture, or suspended by it later (Safari after a call or with the
    // tab in the background: "interrupted"), stays silent until resumed inside a gesture, so every gesture resumes it
    if (s.ctx && s.ctx.state === "closed") { s.ctx = null; s.node = null; s.posted = 0; s.rep = null; }
    if (s.ctx) {
        if (s.ctx.state !== "running") s.ctx.resume().catch(() => {});
        return;
    }
    if (s.failed) return;
    const AC = window.AudioContext || window.webkitAudioContext;
    let ctx = null;
    try {
        ctx = new AC({ sampleRate: 44100, latencyHint: "interactive" });
    } catch (e) {
        try { ctx = new AC(); } catch (e2) { ctx = null; }
    }
    if (!ctx) {
        console.warn("no sound: this browser gave no audio output");
        s.failed = true; // do not try again on every click
        return;
    }
    // iPhone and iPad: play through the ring/silent switch, as a game app does (Safari 16.4+)
    try { if (navigator.audioSession) navigator.audioSession.type = "playback"; } catch (e) {}
    s.ctx = ctx;
    s.rate = ctx.sampleRate;
    s.posted = 0; s.rep = null; s.repAt = 0; s.postAt = 0; s.clock = -1;
    if (ctx.state !== "running") ctx.resume().catch(() => {});
    const scriptProcessor = () => {
        // no AudioWorklet (Safari before 14.1, a page served over plain http from another machine): blocks pulled on this
        // thread, with the largest buffer that keeps the latency acceptable
        if (s.ctx !== ctx || s.node) return;
        const node = ctx.createScriptProcessor(4096, 0, 2);
        node.onaudioprocess = e => {
            const n = e.outputBuffer.length;
            const l = e.outputBuffer.getChannelData(0), r = e.outputBuffer.getChannelData(1);
            const ptr = typeof wasm_exports !== "undefined" && wasm_exports.sg_audio_fill ? wasm_exports.sg_audio_fill(s.rate, n) : 0;
            if (!ptr) { l.fill(0); r.fill(0); return; }
            const src = new Float32Array(wasm_memory.buffer, ptr, n * 2);
            for (let i = 0; i < n; i++) { l[i] = src[2 * i]; r[i] = src[2 * i + 1]; }
        };
        node.connect(ctx.destination);
        s.node = node;
    };
    if (!ctx.audioWorklet || typeof AudioWorkletNode === "undefined") return scriptProcessor();
    const url = URL.createObjectURL(new Blob([SG_WORKLET], { type: "application/javascript" }));
    ctx.audioWorklet.addModule(url).then(() => {
        if (s.ctx !== ctx) return;
        const node = new AudioWorkletNode(ctx, "sg-out", { numberOfInputs: 0, numberOfOutputs: 1, outputChannelCount: [2] });
        node.port.onmessage = e => {
            s.rep = e.data;
            s.repAt = performance.now();
            if (e.data[3]) {
                s.under++;
                // the game's frames are longer here than the lead covers: keep more ready for a while (not for a tab in
                // the background, whose timers the browser slows down)
                if (!document.hidden) s.lead = Math.min(SG_LEAD_MAX, s.lead + 0.05);
                s.calm = performance.now();
            } else if (s.lead > SG_LEAD && performance.now() - s.calm > 10000) {
                // ten seconds without a gap: back towards the short lead (a click's sound follows the click sooner)
                s.lead = Math.max(SG_LEAD, s.lead - 0.05);
                s.calm = performance.now();
            }
            sgPump();
        };
        node.connect(ctx.destination);
        s.node = node;
        console.log("sound: output at " + ctx.sampleRate + " Hz (AudioWorklet), " + ctx.state);
        sgPump();
    }).catch(e => {
        console.warn("sound: no AudioWorklet, using a ScriptProcessorNode", e);
        scriptProcessor();
    }).finally(() => URL.revokeObjectURL(url));
}

// Mixes the game's sound up to the lead ahead of what the worklet has played. What it still holds is reckoned from its last
// report and the audio clock since (its reports wait while this thread is busy, the clock does not), plus what is on the
// way to it.
function sgPump() {
    const s = sgSnd;
    if (!s.node || !s.node.port || s.ctx.state !== "running" || typeof wasm_exports === "undefined" || !wasm_exports.sg_audio_fill) return;
    const queued = s.rep ? Math.max(0, s.posted - s.rep[0] + s.rep[1] - (s.ctx.currentTime * s.rate - s.rep[2])) : s.posted;
    let need = Math.round((document.hidden ? SG_LEAD_HIDDEN : s.lead) * s.rate) - queued;
    while (need >= 256) {
        const n = Math.min(need, 4096);
        const ptr = wasm_exports.sg_audio_fill(s.rate, n);
        if (!ptr) return;
        const buf = new Float32Array(n * 2);
        buf.set(new Float32Array(wasm_memory.buffer, ptr, n * 2));
        s.node.port.postMessage(buf, [buf.buffer]);
        if (!s.postAt) s.postAt = performance.now();
        s.posted += n;
        need -= n;
    }
}

// Once the game runs, sound that cannot start (no gesture counted yet, a context the browser keeps suspended, no audio
// output at all) gets one note over the game; it goes when the sound plays.
function sgSoundNote() {
    const s = sgSnd;
    const canvas = document.getElementById("glcanvas");
    const playing = s.ctx && s.ctx.state === "running";
    if (s.note && (playing || s.shown === 2)) {
        s.note.remove();
        s.note = null;
        s.shown = 2;
    }
    if (s.shown || playing || !canvas || canvas.style.display !== "block" || typeof wasm_exports === "undefined") return;
    if (!s.since) s.since = performance.now();
    if (performance.now() - s.since < 1500) return;
    s.shown = 1;
    s.note = document.createElement("div");
    s.note.textContent = s.failed ? "No sound: this browser gave the page no audio output."
        : "\u{1F507} Click anywhere or press a key to turn on the sound";
    s.note.style.cssText = "position:fixed;left:50%;top:12px;transform:translateX(-50%);z-index:10;pointer-events:none;" +
        "background:rgba(0,0,0,0.78);color:#fff;font:600 15px system-ui,sans-serif;padding:8px 16px;border-radius:999px";
    document.body.appendChild(s.note);
    if (s.failed) setTimeout(() => { s.shown = 2; }, 8000);
}

// Sound that stopped coming out with no word: a context that says "running" while its clock stands still, or a worklet
// that takes the game's blocks and no longer plays them (reports nothing for seconds). The output is made again (the next
// gesture resumes it if the browser wants one, with the note) and the console says so. Not while this thread was held up
// (a long frame or a tab in the background): the worklet's reports wait for it.
function sgAudioWatch() {
    const s = sgSnd, now = performance.now();
    const held = s.tick && now - s.tick > 500;
    s.tick = now;
    if (!s.ctx || s.ctx.state !== "running" || document.hidden || held) { s.clock = -1; return; }
    if (s.ctx.currentTime !== s.clock) { s.clock = s.ctx.currentTime; s.clockAt = now; }
    const stuckClock = now - s.clockAt > 3000;
    const isWorklet = typeof AudioWorkletNode !== "undefined" && s.node instanceof AudioWorkletNode;
    const deaf = isWorklet && s.postAt && now - Math.max(s.repAt, s.postAt) > 3000;
    if (!stuckClock && !deaf) return;
    if (s.restarts >= 5) return; // not forever: the note and the console have said it
    s.restarts++;
    console.warn("sound: the audio output stopped (" + (stuckClock ? "its clock stands still" : "it plays nothing it is given") +
        "); starting it again");
    const old = s.ctx;
    s.ctx = null; s.node = null;
    try { old.close(); } catch (e) {}
    s.shown = 0; s.since = 0;
    sgStartAudio();
}

// the timer keeps a tab in the background fed; once a frame as well, as a browser busy drawing can hold timers back
setInterval(() => { sgPump(); sgSoundNote(); sgAudioWatch(); }, 20);
(function frame() { sgPump(); requestAnimationFrame(frame); })();
for (const type of ["mousedown", "pointerdown", "pointerup", "click", "keydown", "touchend"]) window.addEventListener(type, sgStartAudio, true);
window.addEventListener("touchstart", sgStartAudio, { passive: true, capture: true });
document.addEventListener("visibilitychange", () => {
    if (!document.hidden && sgSnd.ctx && sgSnd.ctx.state !== "running" && sgSnd.ctx.state !== "closed") sgSnd.ctx.resume().catch(() => {});
    sgPump();
});

// The keyboard while the game runs. miniquad's gl.js sends the canvas's keys to the game and stops the browser acting on
// Space, the arrows, F1 to F10, Tab, Backspace, ' and /. Here the rest of the original's keys are kept from the browser too:
// PageUp and Home (turn the view; they would scroll the page), End and PageDown, and Alt or Ctrl with a digit (the port's
// stand-ins for F1 to F10 on keyboards whose function keys need Fn; Alt+digit switches tabs in some browsers). Alt alone
// would open Firefox's menu bar on Windows. The apostrophe (the measuring tape) opens Firefox's quick find. A key pressed while the page has lost the canvas's focus (after a click on
// the page around it, or back from another window) goes to the game and gives the canvas the focus back. The phone
// keyboard's text box (sgKeyboard) keeps its own keys.
function sgGameKey(e) {
    if (e.metaKey) return false; // Cmd shortcuts stay the browser's (the game needs none)
    if (["PageUp", "PageDown", "Home", "End", "AltLeft", "AltRight", "Quote"].includes(e.code)) return true;
    return (e.altKey || e.ctrlKey) && /^Digit[0-9]$/.test(e.code);
}
for (const type of ["keydown", "keyup"]) {
    window.addEventListener(type, e => {
        const canvas = document.getElementById("glcanvas");
        if (!canvas || canvas.style.display !== "block" || typeof wasm_exports === "undefined") return;
        if (sgKbd && document.activeElement === sgKbd) return;
        if (type === "keydown" && sgGameKey(e)) e.preventDefault();
        if (e.target !== canvas) {
            canvas.focus();
            const handler = type === "keydown" ? canvas.onkeydown : canvas.onkeyup;
            if (handler) handler(e);
        }
    }, true);
}

// The wheel. gl.js hands the game the browser's raw deltas: about 100 pixels a notch from a mouse (in lines on some
// Firefox setups), and a stream of small ones from a Mac trackpad, where the desktop builds get one per notch. Here they are
// turned into notches (a mouse notch is 1, a trackpad's small turns fractions of one) before the game sees them, and the
// page's own scrolling and pinch zoom stay off.
window.addEventListener("wheel", e => {
    const canvas = document.getElementById("glcanvas");
    if (!canvas || e.target !== canvas || typeof wasm_exports === "undefined") return;
    e.preventDefault();
    e.stopPropagation();
    const unit = e.deltaMode === 1 ? 1 / 3 : e.deltaMode === 2 ? 1 : 1 / 100;
    const notch = v => Math.max(-1, Math.min(1, -v * unit));
    wasm_exports.mouse_wheel(notch(e.deltaX), notch(e.deltaY));
}, { capture: true, passive: false });

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

// Controllers (crates/simgolf/src/gamepad.rs): each frame the first connected gamepad's buttons (standard mapping, as
// bits) and sticks go to the game; a controller that goes away is let go once with everything released.
let sgPadWas = false;
function sgPadPoll() {
    requestAnimationFrame(sgPadPoll);
    if (typeof wasm_exports === "undefined" || !wasm_exports.sg_gamepad || !navigator.getGamepads) return;
    const p = Array.from(navigator.getGamepads()).find(g => g && g.connected);
    if (!p) {
        if (sgPadWas) wasm_exports.sg_gamepad(0, 0, 0, 0, 0);
        sgPadWas = false;
        return;
    }
    sgPadWas = true;
    let b = 0;
    for (let i = 0; i < 16 && i < p.buttons.length; i++) if (p.buttons[i].pressed) b |= 1 << i;
    const a = p.axes;
    wasm_exports.sg_gamepad(b, a[0] || 0, a[1] || 0, a[2] || 0, a[3] || 0);
}
requestAnimationFrame(sgPadPoll);
