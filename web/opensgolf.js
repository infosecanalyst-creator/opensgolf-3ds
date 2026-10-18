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
//
// Whether a frame is due is decided once per animation frame (its timestamp) and holds for every callback in it. Issue
// #55: the decision used to be made per callback, and the page's own loops (sgPump, sgPadPoll), asked for before the
// game's and so run first in every frame, took each due frame for themselves: with any cap set the game never drew
// again after its first frame, in Classic too (the cap counts there), and every reload came up frozen on the title. The
// page's own loops also ask the browser directly (sgRequestFrame), so only the game's drawing is capped.
let sgFrameCap = 0;
let sgFrameDue = 0;
let sgFrameAt = null;
let sgFrameRun = true;
const sgRequestFrame = window.requestAnimationFrame.bind(window);
// Whether the animation frame at `t` (its timestamp) is drawn under the cap.
function sgFrameOn(t) {
    if (sgFrameCap <= 0) return true;
    if (t !== sgFrameAt) {
        sgFrameAt = t;
        const period = 1000 / sgFrameCap;
        const late = t - sgFrameDue;
        // a millisecond early still counts, so a 60 cap on a 60 Hz display skips nothing; a due time more than a period
        // away (the cap changed, a clock that went back) is not waited for
        sgFrameRun = late >= -1 || late < -period;
        if (sgFrameRun) sgFrameDue = late >= -1 && late < period ? sgFrameDue + period : t + period;
    }
    return sgFrameRun;
}
window.requestAnimationFrame = function (callback) {
    const tick = t => {
        if (!sgFrameOn(t)) return sgRequestFrame(tick);
        try {
            callback(t);
        } catch (e) {
            // miniquad asks for the next frame after drawing this one: a frame that throws ends the game's drawing for good
            sgDrawingStopped("a frame stopped with an error: " + (e && e.message ? e.message : e));
            throw e;
        }
    };
    return sgRequestFrame(tick);
};

// The watch over the game's drawing (issue #55). A browser that loses the game's WebGL drawing (the graphics driver
// reset, the GPU process restarted, a frame that stopped with an error) shows the last picture for good while the sound,
// mixed on timers, plays on, and the clicks still reach the game: only the title's exit button (which reloads the page)
// seemed to work. Then the page says so over the game and in the console and leaves the marker of a start that failed
// (crates/simgolf/src/enhanced.rs), so the next start comes up safe: Classic, and no frame rate cap (the cap counts in
// Classic too, and a saved setting that stops the drawing in Classic would otherwise bring the freeze back at every
// start). The game reports each frame drawn (sg_frame_drawn) and whether Enhanced is on (sg_enhanced).
const SG_TRIAL_KEY = "enhanced_starting";
const SG_STALL_MS = 5000;
const sgDraw = { enhanced: false, last: 0, tick: 0, stopped: false, note: null };

function sgDrawingStopped(why) {
    const d = sgDraw;
    const canvas = document.getElementById("glcanvas");
    if (d.stopped || !canvas || canvas.style.display !== "block") return;
    d.stopped = true;
    try { localStorage.setItem(SAVE_PREFIX + SG_TRIAL_KEY, btoa("1")); } catch (e) {}
    console.error("OpenSGolf: the browser stopped drawing the game (" + why + "); the next start uses safe graphics " +
        "settings (Classic, no frame rate cap" + (sgFrameCap > 0 ? ", was " + sgFrameCap : "") + ")" +
        (d.enhanced ? ", Enhanced graphics were on" : ""));
    const note = document.createElement("div");
    note.style.cssText = "position:fixed;left:50%;top:50%;transform:translate(-50%,-50%);z-index:20;max-width:min(560px,90vw);" +
        "background:rgba(10,30,18,0.94);color:#fff;font:500 16px/1.45 system-ui,sans-serif;padding:20px 24px;border-radius:14px;" +
        "box-shadow:0 12px 40px rgba(0,0,0,0.6);text-align:center";
    const text = document.createElement("p");
    text.style.margin = "0 0 14px";
    text.textContent = "The browser stopped drawing the game, so the picture froze. The graphics settings go back to " +
        "safe ones for the next start (Classic, no frame rate cap): reload the page to play on. Games you saved are kept.";
    const button = document.createElement("button");
    button.textContent = "Reload the page";
    button.style.cssText = "font:700 16px system-ui,sans-serif;padding:10px 22px;border-radius:999px;border:0;cursor:pointer;" +
        "background:#f4d35e;color:#10240f";
    button.addEventListener("click", () => location.reload());
    note.append(text, button);
    document.body.appendChild(note);
    d.note = note;
}

// No frame for SG_STALL_MS while the page's timers run and the tab is shown: the game's drawing has stopped. Not before
// the first frame, not in a hidden tab (the browser draws nothing there), and not after a long frame held this thread up
// (the timers waited too).
function sgDrawWatch() {
    const d = sgDraw, now = performance.now();
    const held = d.tick && now - d.tick > 1000;
    d.tick = now;
    if (!d.last || d.stopped) return;
    if (held || document.hidden) {
        d.last = now;
        return;
    }
    if (now - d.last > SG_STALL_MS) sgDrawingStopped("no frame drawn for " + Math.round((now - d.last) / 1000) + " seconds");
}
setInterval(sgDrawWatch, 250);
document.addEventListener("visibilitychange", () => { if (sgDraw.last) sgDraw.last = performance.now(); });
// Leaving a game that was drawing well (a reload, the tab closed, the title's exit button): the marker of Enhanced's
// start-up trial goes, so a player who leaves within its first seconds keeps Enhanced next time.
window.addEventListener("pagehide", () => {
    const d = sgDraw;
    if (d.stopped || !d.last || performance.now() - d.last > 2000) return;
    try {
        const k = sgStoredKey(SG_TRIAL_KEY);
        if (k !== null) localStorage.removeItem(k);
    } catch (e) {}
});

miniquad_add_plugin({
    name: "simgolf_fs",
    version: 1,
    register_plugin: function (importObject) {
        const env = importObject.env;
        env.sg_keyboard = function (show) { sgKeyboard(show !== 0); };
        // the frame rate cap (Enhanced graphics > Display; framerate.rs): frames a second, 0 for none
        env.sg_frame_cap = function (fps) { sgFrameCap = fps; };
        // the watch over the game's drawing (sgDrawingStopped): a frame was drawn; Enhanced graphics on or off
        env.sg_frame_drawn = function () { sgDraw.last = performance.now(); };
        env.sg_enhanced = function (on) { sgDraw.enhanced = on !== 0; };
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
            // Load Previous Game's Export this save (files_ui.rs) writes under download/: a download, kept nowhere
            if (low.startsWith("download/")) {
                sgDownload(path.split(/[\\/]/).pop(), bytes);
                return;
            }
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
        // The course editor's "Import heightmap..." (crates/simgolf/src/editor.rs): the page's file picker for a PNG. The
        // picture is kept under picked/ and its name handed over when the game asks (-1 until one is picked).
        env.sg_pick_file = function () { sgPickFile(); };
        // Load Previous Game's Import a save file (files_ui.rs): the same picker for a saved game, shared course, course
        // code or the original's top10.sve; the game reads it from picked/ and files it as a dropped file (share.rs)
        env.sg_pick_save = function () { sgPickFile(".sgs,.sgch,.txt,.sve"); };
        env.sg_pick_poll = function (dst, cap) {
            if (sgPicked === null) return -1;
            const b = enc.encode(sgPicked);
            sgPicked = null;
            const n = Math.min(b.length, cap);
            new Uint8Array(wasm_memory.buffer, dst, n).set(b.subarray(0, n));
            return n;
        };
    },
});

const sgnet = { next: 1, reqs: new Map() };

// The course editor's heightmap picker: the name of the picture picked, until the game takes it.
let sgPicked = null;
function sgPickFile(accept = ".png,image/png") {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.style.display = "none";
    input.addEventListener("change", () => {
        const f = input.files && input.files[0];
        input.remove();
        if (!f) return;
        f.arrayBuffer().then(b => {
            const name = f.name.replace(/[\\/]/g, "_");
            sgAddFile("picked/" + name, new Uint8Array(b));
            sgPicked = name;
        });
    });
    document.body.appendChild(input);
    input.click();
}

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
// above it, or a .zip of it (web/files.js reads the zip and says where each file goes). The files are kept in this browser
// (IndexedDB) so the next visit starts at once; "Forget my game files" removes them.
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
// The marker of a store being written (sgStoreAll): one found at the start is a copy cut short (the tab closed), not used.
const SG_STORE_PARTIAL = "\u0000incomplete";
async function sgStoreAll(entries, store = "files") {
    try {
        // Safari clears a site's storage after weeks without a visit unless the page is allowed to keep it
        if (navigator.storage && navigator.storage.persist) navigator.storage.persist().catch(() => {});
        const db = await sgDb();
        // in parts of up to 32 MB: one transaction of the whole game folder had the browser copy all of it at once, a
        // second copy of the files an iPad's tab may not have room for beside the game
        const tx = (fill, last) => new Promise((ok, fail) => {
            const t = db.transaction(store, "readwrite");
            fill(t.objectStore(store));
            if (last) t.objectStore(store).delete(SG_STORE_PARTIAL);
            t.oncomplete = ok;
            t.onerror = () => fail(t.error);
            // a full storage aborts the transaction
            t.onabort = () => fail(t.error);
        });
        await tx(st => { st.clear(); st.put(1, SG_STORE_PARTIAL); }, entries.length === 0);
        for (let i = 0; i < entries.length;) {
            let j = i, bytes = 0;
            while (j < entries.length && (j === i || bytes + entries[j][1].length <= 32 * 1048576)) bytes += entries[j++][1].length;
            const part = entries.slice(i, j);
            await tx(st => { for (const [path, b] of part) st.put(b, path); }, j === entries.length);
            i = j;
        }
    } catch (e) {
        // (the marker stays: a copy cut short is not used)
        console.warn("could not keep the game files in this browser", e);
        if (store === "files") sgNote("Your game files could not be kept in this browser (" + (e && e.name === "QuotaExceededError" ?
            "not enough storage space" : (e && e.message) || "storage refused") + "): the game plays on, but next time pick them again.", true);
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
                if (c) { out.push([c.key, c.value]); c.continue(); } else ok(out.some(([k]) => k === SG_STORE_PARTIAL) ? [] : out);
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

// A note over the game, in large type so it is read on an iPad: for 20 seconds, or until tapped for a problem; a tap closes
// it and goes no further.
function sgNote(text, problem) {
    const note = document.createElement("div");
    note.style.cssText = "position:fixed;left:50%;top:calc(12px + env(safe-area-inset-top));transform:translateX(-50%);z-index:20;" +
        "max-width:min(720px,calc(100vw - 32px));background:rgba(10,30,18,0.95);color:#fff;font:600 clamp(16px,2.4vw,20px)/1.45 system-ui,sans-serif;" +
        "padding:14px 20px;border-radius:12px;box-shadow:0 8px 30px rgba(0,0,0,0.5);text-align:center;cursor:pointer" +
        (problem ? ";border:2px solid #f4d35e" : "");
    note.textContent = text + " (Tap to close.)";
    const close = e => { e.stopPropagation(); e.preventDefault(); note.remove(); };
    for (const type of ["pointerdown", "touchstart", "mousedown", "click"]) note.addEventListener(type, close);
    document.body.appendChild(note);
    if (!problem) setTimeout(() => note.remove(), 20000);
}

// --- the saved games: backing up and restoring -------------------------------------------------------------------------------
// What the game writes (saved games, championship courses and pros, settings.json, the Top 10 table, the accomplishment
// snapshots) is kept in this browser's local storage under SAVE_PREFIX, by the file's path in the save folder. "Back up my
// saves" hands all of it over as one .zip; "Restore saves..." takes such a zip (or a zip of a desktop save folder) or single
// save files and puts them back where the game looks (docs/PLAYING.md, "Saved games in the browser").
function sgSaveStatus(t) {
    const el = document.getElementById("savestatus");
    if (el) el.textContent = t;
}
function sgB64(bytes) {
    let bin = "";
    for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
    return btoa(bin);
}
function sgUnB64(text) {
    const bin = atob(text);
    const bytes = new Uint8Array(bin.length);
    for (let j = 0; j < bin.length; j++) bytes[j] = bin.charCodeAt(j);
    return bytes;
}
// a marker of a start in progress (enhanced.rs), not a file to keep
function sgTransient(low) {
    return low === "enhanced_starting";
}
// Every kept file: [path, bytes], sorted by path.
function sgSavedFiles() {
    const out = [];
    for (let i = 0; i < localStorage.length; i++) {
        const k = localStorage.key(i);
        if (!k || !k.startsWith(SAVE_PREFIX)) continue;
        const path = k.substring(SAVE_PREFIX.length);
        if (sgTransient(sgKey(path))) continue;
        try { out.push([path, sgUnB64(localStorage.getItem(k))]); } catch (e) {}
    }
    return out.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
}
// The key a file is kept under, whatever the case of its name (lookups are case-insensitive, like the game's), or null.
function sgStoredKey(path) {
    const k = sgKey(path);
    for (let i = 0; i < localStorage.length; i++) {
        const lk = localStorage.key(i);
        if (lk && lk.startsWith(SAVE_PREFIX) && sgKey(lk.substring(SAVE_PREFIX.length)) === k) return lk;
    }
    return null;
}
// Keeps a file the way the game's own writes are kept (sg_fs_write), in place of any copy of it under another case. False
// when local storage is full.
function sgKeepFile(path, bytes) {
    sgAddFile(path, bytes);
    try {
        for (let old = sgStoredKey(path); old !== null && old !== SAVE_PREFIX + path; old = sgStoredKey(path)) localStorage.removeItem(old);
        localStorage.setItem(SAVE_PREFIX + path, sgB64(bytes));
        return true;
    } catch (e) {
        console.warn("could not keep " + path + " in local storage", e);
        return false;
    }
}

// A .zip with its files stored (not compressed again: the saves are compressed already), names in UTF-8.
function sgZip(entries) {
    const now = new Date();
    const time = (now.getHours() << 11) | (now.getMinutes() << 5) | (now.getSeconds() >> 1);
    const date = ((now.getFullYear() - 1980) << 9) | ((now.getMonth() + 1) << 5) | now.getDate();
    const parts = [], central = [];
    let off = 0;
    for (const [path, data] of entries) {
        const name = enc.encode(path.replace(/\\/g, "/"));
        const crc = sgCrc32(data);
        const l = new DataView(new ArrayBuffer(30));
        l.setUint32(0, 0x04034b50, true);
        l.setUint16(4, 20, true); l.setUint16(6, 0x800, true); l.setUint16(8, 0, true);
        l.setUint16(10, time, true); l.setUint16(12, date, true); l.setUint32(14, crc, true);
        l.setUint32(18, data.length, true); l.setUint32(22, data.length, true); l.setUint16(26, name.length, true);
        const c = new DataView(new ArrayBuffer(46));
        c.setUint32(0, 0x02014b50, true);
        c.setUint16(4, 20, true); c.setUint16(6, 20, true); c.setUint16(8, 0x800, true); c.setUint16(10, 0, true);
        c.setUint16(12, time, true); c.setUint16(14, date, true); c.setUint32(16, crc, true);
        c.setUint32(20, data.length, true); c.setUint32(24, data.length, true); c.setUint16(28, name.length, true);
        c.setUint32(42, off, true);
        parts.push(new Uint8Array(l.buffer), name, data);
        central.push(new Uint8Array(c.buffer), name);
        off += 30 + name.length + data.length;
    }
    const csize = central.reduce((n, b) => n + b.length, 0);
    const e = new DataView(new ArrayBuffer(22));
    e.setUint32(0, 0x06054b50, true);
    e.setUint16(8, entries.length, true); e.setUint16(10, entries.length, true);
    e.setUint32(12, csize, true); e.setUint32(16, off, true);
    return new Blob([...parts, ...central, new Uint8Array(e.buffer)], { type: "application/zip" });
}

function sgIsGame(path) {
    const k = sgKey(path);
    return k.startsWith("saved games/") && k.endsWith(".sgs");
}

function sgBackupSaves() {
    const files = sgSavedFiles();
    if (files.length === 0) {
        sgSaveStatus("No saved games in this browser yet: nothing to back up.");
        return;
    }
    const d = new Date();
    const pad = n => String(n).padStart(2, "0");
    const name = "opensgolf-saves-" + d.getFullYear() + "-" + pad(d.getMonth() + 1) + "-" + pad(d.getDate()) + ".zip";
    const url = URL.createObjectURL(sgZip(files));
    const a = document.createElement("a");
    a.href = url;
    a.download = name;
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 10000);
    const games = files.filter(([p]) => sgIsGame(p)).length;
    sgSaveStatus("Downloaded " + name + ": " + sgPlural(games, "saved game") + " and " + sgPlural(files.length - games, "other file") +
        " (settings, championship courses, records). Keep it somewhere safe; Restore saves... brings it back.");
}

const SG_SAVE_MAGIC = "SIMGOLF-NATIVE-SAVE\n";
function sgIsSave(bytes) {
    if (bytes.length < SG_SAVE_MAGIC.length) return false;
    for (let i = 0; i < SG_SAVE_MAGIC.length; i++) if (bytes[i] !== SG_SAVE_MAGIC.charCodeAt(i)) return false;
    return true;
}
// A free name in a folder for a single file brought in: "Name.sgs", else "Name (2).sgs"... (a different file of the same
// name stays). "" when that very file is there already.
function sgFreeName(dir, file, bytes) {
    file = file.replace(/[\\/:*?"<>|]/g, "_");
    const dot = file.lastIndexOf(".");
    const stem = (dot > 0 ? file.substring(0, dot) : file).replace(/^[&.]+/, "") || "Imported";
    const ext = dot > 0 ? file.substring(dot) : "";
    const b64 = sgB64(bytes);
    for (let n = 1; n < 100; n++) {
        const path = dir + stem + (n === 1 ? "" : " (" + n + ")") + ext;
        const old = sgStoredKey(path);
        if (old === null) return path;
        if (localStorage.getItem(old) === b64) return "";
    }
    return null;
}

// What a path in a restored zip becomes in the save folder, or null to leave it out: the files of a desktop save folder
// that are not saves (the game's own copy, the HD pack, photos, translations, the crash and sound reports) stay out; packs
// go with the content packs.
function sgRestorePath(rel) {
    const low = sgKey(rel);
    if (low === "" || sgTransient(low) || sgSkip(low)) return null;
    const top = low.split("/")[0];
    if (["game", "hd", "photos", "lang", "picked", "download"].includes(top)) return null;
    if (["crash.txt", "sound.txt", "opensgolf-shot.png"].includes(low)) return null;
    if (low.endsWith(".png") && !/(^|\/)accomp\d+\.png$/.test(low)) return null;
    return rel;
}

// The files of a zip to restore: [path in the save folder, bytes]. A backup from this page has them at its top; a zip of a
// desktop save folder may hold them under the folder's name (whatever it was zipped from: the files sit beside "Saved
// Games" and settings.json).
async function sgZipSaves(buf) {
    const entries = [];
    await sgUnzip(buf, async (name, b) => { entries.push([name.replace(/\\/g, "/"), b]); });
    let root = null;
    for (const [name] of entries) {
        const parts = name.split("/");
        const i = parts.findIndex((p, k) => (p.toLowerCase() === "saved games" && k < parts.length - 1) ||
            (p.toLowerCase() === "settings.json" && k === parts.length - 1));
        if (i >= 0) {
            const r = parts.slice(0, i).join("/");
            if (root === null || r.length < root.length) root = r;
        }
    }
    const out = [];
    for (const [name, b] of entries) {
        if (root && !name.startsWith(root + "/")) continue;
        const rel = sgRestorePath(root ? name.substring(root.length + 1) : name);
        if (rel !== null) out.push([rel, b]);
    }
    return out;
}

// "Restore saves...": backups (.zip) and single files (.sgs saved games, .sgch championship courses, .pro pros, the
// original's top10.sve, settings.json). A backup's files replace those of the same name; a single file never replaces one.
async function sgRestoreFiles(fileList) {
    const files = Array.from(fileList || []);
    if (files.length === 0) return;
    let kept = 0, games = 0, full = 0, same = 0;
    const packs = [], original = [], unknown = [];
    const keep = (path, bytes) => {
        if (sgKeepFile(path, bytes)) {
            kept++;
            if (sgIsGame(path)) games++;
        } else full++;
    };
    for (const f of files) {
        const bytes = new Uint8Array(await f.arrayBuffer());
        const low = f.name.toLowerCase();
        if (bytes.length >= 4 && bytes[0] === 0x50 && bytes[1] === 0x4b && bytes[2] === 3 && bytes[3] === 4) {
            let entries;
            try {
                entries = await sgZipSaves(bytes.buffer);
            } catch (e) {
                unknown.push(f.name + " (" + e.message + ")");
                continue;
            }
            if (entries.length === 0) unknown.push(f.name + " (no saves in it)");
            for (const [rel, b] of entries) {
                if (sgKey(rel).startsWith("packs/")) {
                    sgAddFile(rel, b);
                    packs.push([rel, b]);
                } else keep(rel, b);
            }
            continue;
        }
        let path = null;
        if (low === "top10.sve" && bytes.length === 1560) {
            // the original SimGolf's Top 10 Designers (the same layout as the port's)
            if (sgStoredKey("top10.sve") !== null && !confirm("Replace this browser's Top 10 Designers with the table in " + f.name +
                "? (Import a save file... on the game's Load Previous Game adds it to yours instead.)")) continue;
            path = "top10.sve";
        } else if (low.endsWith(".sve")) {
            original.push(f.name);
            continue;
        } else if (low.endsWith(".sgs") && sgIsSave(bytes)) {
            path = sgFreeName("Saved Games/", f.name, bytes);
        } else if ((low.endsWith(".sgch") && sgIsSave(bytes)) || low.endsWith(".pro")) {
            path = sgFreeName("Championship/", f.name, bytes);
        } else if (low === "settings.json") {
            path = "settings.json";
        } else {
            unknown.push(f.name);
            continue;
        }
        if (path === "") same++;
        else if (path === null) full++;
        else keep(path, bytes);
    }
    if (packs.length > 0) await sgStoreAdd(packs, "packs");
    const say = [];
    if (kept > 0 || packs.length > 0) {
        say.push("Restored " + sgPlural(kept, "file") + " (" + sgPlural(games, "saved game") + ")" +
            (packs.length ? " and " + sgPlural(packs.length, "content pack file") : "") + ". The games are in Load Previous Game.");
    }
    if (same > 0) say.push(sgPlural(same, "file") + " already in this browser.");
    if (full > 0) say.push(sgPlural(full, "file") + " did not fit in this browser's storage; delete old saves in the game and try again.");
    if (original.length > 0) say.push(original.join(", ") + ": saved games of the original SimGolf, which OpenSGolf cannot read yet (their layout is not decoded).");
    if (unknown.length > 0) say.push(unknown.join(", ") + ": not a saved game or a backup (left out).");
    sgSaveStatus(say.join(" ") || "Nothing to restore.");
    sgSaveCount();
}

// The saved games kept in this browser, on the start page.
function sgSaveCount() {
    const el = document.getElementById("savecount");
    if (!el) return;
    const files = sgSavedFiles();
    const games = files.filter(([p]) => sgIsGame(p)).length;
    el.textContent = files.length === 0 ? "No saved games in this browser yet."
        : "Kept in this browser: " + sgPlural(games, "saved game") + " (" + sgPlural(files.length, "file") + " in all).";
}

// --- starting --------------------------------------------------------------------------------------------------------------
function sgStart() {
    sgRestoreSaves();
    // ?classic in the address starts with safe graphics settings (Classic, no frame rate cap): the same marker a run that
    // stopped drawing leaves behind (crates/simgolf/src/enhanced.rs), so the game puts them back and says so (issue #55)
    if (new URLSearchParams(location.search).has("classic")) sgAddFile("enhanced_starting", new Uint8Array([49]));
    document.getElementById("picker").style.display = "none";
    const canvas = document.getElementById("glcanvas");
    // a right-click (or a Mac's Ctrl+click) is the game's: no browser menu over the canvas, which would also take the
    // button's release
    canvas.addEventListener("contextmenu", e => e.preventDefault());
    // the graphics driver reset or the browser's GPU process restarted: the game cannot make its art again
    canvas.addEventListener("webglcontextlost", () => sgDrawingStopped("the WebGL context was lost"));
    canvas.style.display = "block";
    // the page under the game must not scroll (Safari moves a fixed canvas with it while its bars slide)
    document.documentElement.style.overflow = "hidden";
    document.body.style.overflow = "hidden";
    sgFitCanvas();
    canvas.focus();
    // the sound output (see sgStartAudio)
    sgSnd.gameOn = true;
    sgStartAudio();
    load("opensgolf.wasm");
}

// The game's canvas covers what the player can see of the page: the visual viewport. iPad and iPhone Safari's 100vh (and
// the window's height at load) count the space under the address, tab and bottom bars as well, so the bottom of the game
// (the dock, the lower row of a panel) was cut off while the bars showed. Sized again when the bars show or hide, the
// device turns, or the window changes; the game lays itself out for the new size (miniquad's resize).
function sgFitCanvas() {
    const c = document.getElementById("glcanvas");
    if (!c || c.style.display !== "block") return;
    const vv = window.visualViewport;
    const w = vv ? vv.width : window.innerWidth, h = vv ? vv.height : window.innerHeight;
    const css = (k, v) => { if (c.style[k] !== v) c.style[k] = v; };
    css("width", Math.round(w) + "px");
    css("height", Math.round(h) + "px");
    css("left", Math.round(vv ? vv.offsetLeft : 0) + "px");
    css("top", Math.round(vv ? vv.offsetTop : 0) + "px");
    // gl.js's own resize (it listens to the window's resize only, which Safari does not send for its bars)
    if (typeof resize === "function" && typeof wasm_exports !== "undefined" && wasm_exports.resize) resize(c, wasm_exports.resize);
}
if (window.visualViewport) {
    window.visualViewport.addEventListener("resize", sgFitCanvas);
    window.visualViewport.addEventListener("scroll", sgFitCanvas);
}
window.addEventListener("resize", sgFitCanvas);
// (the new size comes a moment after the turn on older iOS, and a bar sliding away does not always send an event)
window.addEventListener("orientationchange", () => { sgFitCanvas(); setTimeout(sgFitCanvas, 300); });
setInterval(sgFitCanvas, 500);

// No game folder: the game finds no original files under game/ and plays on the free base set (opensgolf-free), which it
// draws itself.
function sgPlayFree() {
    sgStatus("Starting with the free OpenSGolf graphics...");
    sgStart();
}

const SG_ISO_HELP = "That is a Mac disk image (.dmg), which the browser cannot read. Make a .cdr instead: with the CD in, open " +
    "Disk Utility, File > New Image > Image from the CD, Format: DVD/CD master; then pick the .cdr here.";
// a CD image by its contents, whatever its name: ISO 9660's "CD001" at sector 16 (as the native importer looks, iso.rs)
async function sgIsCdImage(file) {
    if (file.size < 16 * 2048 + 6) return false;
    const b = new Uint8Array(await file.slice(16 * 2048 + 1, 16 * 2048 + 6).arrayBuffer());
    return String.fromCharCode(...b) === "CD001";
}
// Plays with the player's files: [path, bytes] (a folder's, a zip's or the CD's). `skipped` is what sgUnzip could not
// unpack, `stat` what it met (web/files.js).
async function sgUse(entries, skipped = [], stat = {}) {
    const place = sgPlace(entries.map(e => e[0]));
    if (place === null && entries.some(e => /\.dmg$/i.test(e[0]))) {
        sgStatus(SG_ISO_HELP);
        return;
    }
    if (place === null) {
        sgStatus("That does not look like a SimGolf game folder (no Flics folder found). Pick the folder that contains Flics, Sounds and Data, or a .zip of it.");
        return;
    }
    const keep = [];
    for (const [i, path] of place) {
        const rel = "game/" + path;
        sgAddFile(rel, entries[i][1]);
        keep.push([rel, entries[i][1]]);
    }
    // the sound files the player's files held anywhere, used or not
    const found = entries.filter(([p]) => sgIsSound(p)).length + skipped.filter(([n]) => sgIsSound(n)).length;
    const sum = sgFilesSummary(keep.map(([p]) => p), skipped, found);
    const how = stat.methods ? Object.entries(stat.methods).map(([m, n]) => n + " " + m).join(", ") : "";
    console.log("files: " + sum.text + (how ? " (zip: " + how + (stat.fallback ? "; " + stat.fallback + " deflated by the page" : "") +
        (stat.crc ? "; " + stat.crc + " with a bad checksum" : "") + ")" : ""));
    sgSummary(sum, true);
    // play at once; the copy kept for next time is written in the background (it can take minutes for a whole game folder)
    const remember = document.getElementById("remember").checked;
    sgStart();
    sgNote(sum.text, sum.problem);
    if (remember) sgStoreAll(keep);
}

// The summary of the player's files in large type on the start page; kept for the next visit (the start page is gone once
// the game runs).
const SG_SUMMARY_KEY = "opensgolf-files-summary";
function sgSummary(sum, keep) {
    const el = document.getElementById("summary");
    if (el) {
        el.textContent = sum.text;
        el.className = sum.problem ? "problem" : "";
    }
    if (keep) try { localStorage.setItem(SG_SUMMARY_KEY, JSON.stringify(sum)); } catch (e) {}
}

async function sgLoadFolder(fileList) {
    const files = Array.from(fileList).filter(f => !sgSkip(f.webkitRelativePath.toLowerCase()));
    if (sgRootOf(files.map(f => f.webkitRelativePath)) === null) {
        // the CD itself (its installer, no installed game): DATA1.HDR and the DATA*.CAB beside it
        const hdr = files.find(f => /(^|\/)data1\.hdr$/i.test(f.webkitRelativePath));
        if (hdr) {
            const dir = hdr.webkitRelativePath.substring(0, hdr.webkitRelativePath.length - hdr.name.length);
            const cabs = new Map();
            for (const f of files) {
                const v = f.webkitRelativePath === dir + f.name ? sgCabNumber(f.name) : null;
                if (v !== null) cabs.set(v, f);
            }
            await sgUseCd("the CD", async (onFile, progress) =>
                sgCabUnpack(new Uint8Array(await hdr.arrayBuffer()), cabs, onFile, progress));
            return;
        }
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
    if (/\.dmg$/i.test(file.name)) {
        sgStatus(SG_ISO_HELP);
        return;
    }
    // an image of the CD (.iso, a Mac's .cdr, or misnamed: known by its contents)
    if (await sgIsCdImage(file)) {
        await sgUseCd(file.name, (onFile, progress) => sgIsoUnpack(file, onFile, progress));
        return;
    }
    if (/\.(bin|cue|img|mdf|nrg)$/i.test(file.name)) {
        sgStatus("That disc image format cannot be read (only an .iso, or a Mac's .cdr). The SimGolf installation folder, or a .zip of it, works too.");
        return;
    }
    // other archives, known by their first bytes: the page reads zips only
    const head = new Uint8Array(await file.slice(0, 6).arrayBuffer());
    const kind = head[0] === 0x37 && head[1] === 0x7a && head[2] === 0xbc && head[3] === 0xaf ? "a 7-Zip archive (.7z)"
        : head[0] === 0x52 && head[1] === 0x61 && head[2] === 0x72 && head[3] === 0x21 ? "a RAR archive" : null;
    if (kind) {
        sgStatus("That is " + kind + ", which the browser cannot unpack. Unpack it on a computer (7-Zip, The Unarchiver) and " +
            "pick the folder, or zip that folder and pick the .zip.");
        return;
    }
    sgStatus("Opening " + file.name + "...");
    let list;
    try {
        list = await sgZipEntries(file);
    } catch (e) {
        sgStatus("Could not read that zip: " + e.message);
        return;
    }
    if (await sgZipCd(file, list)) return;
    const entries = [];
    const skipped = [];
    const stat = {};
    try {
        await sgUnzip(file, async (name, bytes, i, n) => {
            entries.push([name, bytes]);
            if (i % 50 === 0) {
                sgStatus("Unpacking your game files: " + (i + 1) + " of " + n);
                await new Promise(r => setTimeout(r, 0));
            }
        }, skipped, list, stat);
    } catch (e) {
        sgStatus("Could not read that zip: " + e.message + (/memory|allocat/i.test(e.message) ? " (the browser ran out of memory)" : ""));
        return;
    }
    // files it could not unpack are said in the summary over the game (sgFilesSummary), not in an alert, which Safari may
    // not show after the long unpacking; the game still starts (missing sounds are replaced by the free set's)
    await sgUse(entries, skipped, stat);
}

// --- the SimGolf CD in the browser --------------------------------------------------------------------------------------------
// An image of the CD (.iso, or a Mac's Disk Utility .cdr: the same bytes; also inside a .zip), a .zip of the CD's files, or
// the CD itself picked as a folder: the installer's archives on it (DATA1.HDR, DATA*.CAB: InstallShield 6 cabinets) are
// unpacked here into the installed game's files, which then go the way of a picked zip (sgUse). The same reading as the
// desktop and Android import (crates/simgolf/src/iso.rs: ISO 9660/Joliet, the cabinets' file table, split files,
// obfuscated and deflated blocks; layout after unshield's lib/cabfile.h), ported because the page unpacks before the
// game's WebAssembly starts. The image is read a part at a time from the picked file (iPad Safari cannot hold a 215 MB
// image and the game's 240 MB of files at once); videos and Windows programs are left out (sgSkip).
function sgCdError(why) {
    const e = new Error(why);
    e.sgCd = true;
    return e;
}
function sgU16(d, at) {
    if (at + 2 > d.length) throw sgCdError("truncated disc or cabinet record");
    return d[at] | (d[at + 1] << 8);
}
function sgU32(d, at) {
    if (at + 4 > d.length) throw sgCdError("truncated disc or cabinet record");
    return (d[at] | (d[at + 1] << 8) | (d[at + 2] << 16) | (d[at + 3] << 24)) >>> 0;
}
function sgU64(d, at) {
    return sgU32(d, at) + sgU32(d, at + 4) * 4294967296;
}
// A CD image's files: [{ name, offset, size }] (iso.rs Disc::entries).
async function sgIsoEntries(blob) {
    const read = async (offset, size) => {
        if (offset + size > blob.size) throw sgCdError("disc image is cut short");
        return sgSlice(blob, offset, size);
    };
    let root = null, unicode = false;
    for (let sector = 16; sector < 80; sector++) {
        const d = await read(sector * 2048, 2048);
        if (String.fromCharCode(...d.subarray(1, 6)) !== "CD001" || d[6] !== 1) throw sgCdError("not an ISO 9660 disc image");
        if (d[0] === 255) break;
        const esc = String.fromCharCode(...d.subarray(88, 91));
        const joliet = d[0] === 2 && ["%/@", "%/C", "%/E"].includes(esc);
        if (joliet || (d[0] === 1 && root === null)) {
            root = [sgU32(d, 158) * 2048, sgU32(d, 166)];
            unicode = joliet;
        }
    }
    if (root === null) throw sgCdError("ISO has no readable filesystem");
    const stack = [["", root[0], root[1], 0]], visited = new Set(), out = [];
    while (stack.length) {
        const [parent, offset, size, depth] = stack.pop();
        if (visited.has(offset)) continue;
        visited.add(offset);
        if (depth > 32 || size > 16 * 1024 * 1024 || visited.size > 10000) throw sgCdError("ISO directory limits exceeded");
        const data = await read(offset, size);
        let pos = 0;
        while (pos < data.length) {
            const len = data[pos];
            if (len === 0) {
                pos = (Math.floor(pos / 2048) + 1) * 2048;
                continue;
            }
            if (len < 34 || pos + len > data.length) throw sgCdError("invalid ISO directory record");
            const record = data.subarray(pos, pos + len);
            const raw = record.subarray(33, 33 + record[32]);
            pos += len;
            if (raw.length === 1 && raw[0] <= 1) continue;
            let name;
            if (unicode) {
                if (raw.length % 2) throw sgCdError("invalid Joliet filename");
                name = "";
                for (let i = 0; i < raw.length; i += 2) name += String.fromCharCode((raw[i] << 8) | raw[i + 1]);
            } else name = new TextDecoder().decode(raw);
            name = name.split(";")[0].replace(/\.+$/, "");
            if (name === "" || /[\/\\:]/.test(name) || name === "..") throw sgCdError("unsafe ISO filename");
            const full = parent ? parent + "/" + name : name;
            const at = sgU32(record, 2) * 2048, fsize = sgU32(record, 10);
            if (record[25] & 0x80) throw sgCdError("multi-extent ISO files are not supported");
            if (at + fsize > blob.size) throw sgCdError("ISO file extends beyond the disc");
            if (record[25] & 2) stack.push([full, at, fsize, depth + 1]);
            else out.push({ name: full, offset: at, size: fsize });
            if (out.length > 100000) throw sgCdError("too many ISO files");
        }
    }
    return out;
}
// The volume number of a cabinet named like DATA2.CAB, or null.
function sgCabNumber(name) {
    const m = /^data(\d+)\.cab$/i.exec(name);
    return m ? Number(m[1]) : null;
}
// A CP-1252 file name of the installer (the CD's names are Windows').
const SG_CP1252 = "€\u0081‚ƒ„…†‡ˆ‰Š‹Œ\u008dŽ\u008f\u0090‘’“”•–—˜™š›œ\u009džŸ";
function sgCabString(d, at) {
    let s = "";
    for (let i = at; ; i++) {
        if (i >= d.length) throw sgCdError("unterminated cabinet filename");
        const b = d[i];
        if (b === 0) return s;
        s += b >= 128 && b < 160 ? SG_CP1252[b - 128] : String.fromCharCode(b);
    }
}
// A path the game may write: relative, no "..", no drive.
function sgSafeRel(name) {
    const parts = name.split("/").filter(c => c !== "" && c !== ".");
    if (parts.some(c => c === ".." || c.includes(":")) || parts.length === 0) return null;
    return parts.join("/");
}
// The installer's archives: DATA1.HDR's bytes and the cabinets by volume number (Blobs). Calls onFile(path, bytes) for each
// file of the installed game (iso.rs unpack_cabinets).
async function sgCabUnpack(header, cabs, onFile, progress) {
    if (String.fromCharCode(...header.subarray(0, 4)) !== "ISc(") throw sgCdError("invalid InstallShield header");
    const version = sgU32(header, 4);
    const major = version >>> 24 === 1 ? (version >>> 12) & 15 : Math.floor((version & 65535) / 100);
    if (major < 6 || major > 16) throw sgCdError("InstallShield " + major + " is not supported (expected version 6 to 16)");
    const descriptor = sgU32(header, 12);
    const table = descriptor + sgU32(header, descriptor + 12);
    const directories = sgU32(header, descriptor + 28);
    const count = sgU32(header, descriptor + 40);
    const records = table + sgU32(header, descriptor + 44);
    if (count > 100000 || directories > 10000) throw sgCdError("installer file limits exceeded");
    if (records + count * 87 > header.length || table + directories * 4 > header.length) throw sgCdError("truncated installer header");
    const rec = i => header.subarray(records + i * 87, records + i * 87 + 87);
    const items = [];
    for (let i = 0; i < count; i++) {
        let r = rec(i);
        if (sgU16(r, 0) & 8 || sgU64(r, 18) === 0) continue;
        const dir = sgU16(r, 62);
        if (dir >= directories) throw sgCdError("invalid cabinet directory");
        const directory = sgCabString(header, table + sgU32(header, table + dir * 4)).replace(/\\/g, "/");
        const file = sgCabString(header, table + sgU32(r, 58));
        if (/[\/\\:]/.test(file)) throw sgCdError("unsafe cabinet filename");
        const name = directory ? directory + "/" + file : file;
        if (sgSkip(name.toLowerCase())) continue;
        if (name.startsWith("/") || sgSafeRel(name) === null) throw sgCdError("unsafe cabinet path");
        // InstallShield keeps one copy of identical files: a record may point at another's data
        let source = i;
        const links = new Set();
        while (r[84] & 1) {
            source = sgU32(r, 76);
            if (source >= count || links.has(source)) throw sgCdError("invalid or cyclic cabinet file link");
            links.add(source);
            r = rec(source);
        }
        if (sgU16(r, 0) & 8 || sgU64(r, 18) === 0) throw sgCdError("cabinet link refers to an invalid file");
        items.push({ name, flags: sgU16(r, 0), size: sgU64(r, 2), compressed: sgU64(r, 10), offset: sgU64(r, 18), volume: sgU16(r, 85), index: source });
    }
    if (sgRootOf(items.map(it => it.name)) === null) throw sgCdError("no SimGolf game in the installer");
    const heads = new Map();
    const volumeHead = async v => {
        if (!heads.has(v)) {
            const blob = cabs.get(v);
            if (!blob) throw sgCdError("missing data" + v + ".cab");
            const h = await sgSlice(blob, 0, 84);
            if (String.fromCharCode(...h.subarray(0, 4)) !== "ISc(") throw sgCdError("invalid cabinet volume");
            heads.set(v, h);
        }
        return heads.get(v);
    };
    for (let k = 0; k < items.length; k++) {
        const it = items[k];
        if (it.size > 256 * 1024 * 1024 || it.compressed > 256 * 1024 * 1024) throw sgCdError("cabinet file is too large");
        const parts = [];
        let remaining = it.flags & 4 ? it.compressed : it.size, volume = it.volume;
        while (remaining > 0) {
            const h = await volumeHead(volume);
            const blob = cabs.get(volume);
            let offset, available;
            if (it.flags & 1) {
                // a file split over two volumes: the volume's header says where its part is
                if (it.index === sgU32(h, 32)) [offset, available] = [sgU64(h, 60), sgU64(h, it.flags & 4 ? 76 : 68)];
                else if (it.index === sgU32(h, 28)) [offset, available] = [sgU64(h, 36), sgU64(h, it.flags & 4 ? 52 : 44)];
                else throw sgCdError("invalid split cabinet file");
            } else [offset, available] = [it.offset, remaining];
            const n = Math.min(available, remaining);
            if (n === 0 || offset + n > blob.size) throw sgCdError("cabinet file is cut short");
            parts.push(await sgSlice(blob, offset, n));
            remaining -= n;
            volume++;
        }
        let body = parts.length === 1 ? parts[0] : new Uint8Array(await new Blob(parts).arrayBuffer());
        if (it.flags & 2) {
            // the installer's obfuscation
            for (let i = 0; i < body.length; i++) {
                const b = body[i] ^ 0xd5;
                body[i] = (((b >>> 2) | (b << 6)) - (i % 71)) & 255;
            }
        }
        let out;
        if (it.flags & 4) {
            // deflated blocks of up to 64 KiB, each with its length before it
            out = new Uint8Array(it.size);
            let pos = 0, o = 0;
            while (pos < body.length) {
                const len = sgU16(body, pos);
                if (len === 0) throw sgCdError("empty compressed cabinet block");
                if (pos + 2 + len > body.length) throw sgCdError("truncated disc or cabinet record");
                try {
                    o = sgInflateInto(body.subarray(pos + 2, pos + 2 + len), out, o, false);
                } catch (e) {
                    throw sgCdError("damaged cabinet file: " + it.name + " (" + e.message + ")");
                }
                pos += 2 + len;
            }
            if (o !== it.size) throw sgCdError("incorrect extracted size: " + it.name);
        } else {
            if (body.length !== it.size) throw sgCdError("incorrect extracted size: " + it.name);
            out = body;
        }
        await onFile(sgSafeRel(it.name), out);
        await progress(k + 1, items.length);
    }
}
// A CD image (a Blob): the installed game's files, through onFile(path, bytes) (iso.rs unpack).
async function sgIsoUnpack(blob, onFile, progress) {
    const entries = await sgIsoEntries(blob);
    if (sgRootOf(entries.map(e => e.name)) !== null) {
        // an image of an installed game folder: its files as they are
        const wanted = entries.filter(e => !sgSkip(e.name.toLowerCase()));
        for (let i = 0; i < wanted.length; i++) {
            await onFile(wanted[i].name, await sgSlice(blob, wanted[i].offset, wanted[i].size));
            await progress(i + 1, wanted.length);
        }
        return;
    }
    const base = n => n.split("/").pop();
    const header = entries.find(e => base(e.name).toLowerCase() === "data1.hdr");
    if (!header) throw sgCdError("no SimGolf game or installer on this disc");
    if (header.size > 16 * 1024 * 1024) throw sgCdError("installer header is too large");
    const dir = n => n.substring(0, Math.max(0, n.lastIndexOf("/")));
    const cabs = new Map();
    for (const e of entries) {
        const v = dir(e.name) === dir(header.name) ? sgCabNumber(base(e.name)) : null;
        if (v !== null) cabs.set(v, blob.slice(e.offset, e.offset + e.size));
    }
    await sgCabUnpack(await sgSlice(blob, header.offset, header.size), cabs, onFile, progress);
}
// Unpacks the CD from an image or its installer files and plays (sgUse). `from` names it for the messages; `unpack` is
// (onFile, progress) => Promise.
async function sgUseCd(from, unpack) {
    const entries = [];
    try {
        await unpack(async (path, bytes) => { entries.push([path, bytes]); }, async (i, n) => {
            if (i % 25 === 0 || i === n) {
                sgStatus("Unpacking the CD (" + from + "): " + i + " of " + n + " files");
                await new Promise(r => setTimeout(r, 0));
            }
        });
    } catch (e) {
        console.warn("CD:", e);
        sgStatus("Could not unpack " + from + ": " + e.message + (e.sgCd ? "." : " (the browser ran out of memory?). The desktop version can unpack it."));
        return;
    }
    console.log("CD: " + entries.length + " files unpacked from " + from);
    await sgUse(entries);
}

// A zip of the CD rather than of the game: an image in it, or the CD's installer files. True when it was (or was not) one
// and the player was answered.
async function sgZipCd(file, list) {
    const names = list.filter(e => !sgSkip(e.name.toLowerCase())).map(e => e.name);
    if (sgRootOf(names) !== null) return false;
    const blobOf = async e => {
        // a stored entry is a part of the picked file, read as needed; a compressed one is unpacked first
        const at = await sgZipDataAt(file, e);
        const raw = file.slice(at, at + e.csize);
        if (e.method === 0) return raw;
        if (e.method === 8 && typeof DecompressionStream !== "undefined") return new Response(raw.stream().pipeThrough(new DecompressionStream("deflate-raw"))).blob();
        // (the page's own decoders: the whole entry in memory)
        const bytes = await sgZipRead(file, e);
        if (typeof bytes === "string") throw sgCdError(e.name + ": " + bytes);
        return new Blob([bytes]);
    };
    const images = list.filter(e => /\.(iso|cdr)$/i.test(e.name) && !sgSkip(e.name.toLowerCase()));
    if (images.length > 1) {
        sgStatus("That zip holds several CD images; pick the SimGolf one itself.");
        return true;
    }
    if (images.length === 1) {
        await sgUseCd(images[0].name.split("/").pop(), async (onFile, progress) => sgIsoUnpack(await blobOf(images[0]), onFile, progress));
        return true;
    }
    const hdr = list.find(e => /(^|\/)data1\.hdr$/i.test(e.name));
    if (hdr) {
        const dir = hdr.name.substring(0, hdr.name.length - "data1.hdr".length);
        await sgUseCd("the CD's files", async (onFile, progress) => {
            const cabs = new Map();
            for (const e of list) {
                const rest = e.name.substring(dir.length);
                const v = e.name.startsWith(dir) && !rest.includes("/") ? sgCabNumber(rest) : null;
                if (v !== null) cabs.set(v, await blobOf(e));
            }
            const h = await sgZipRead(file, hdr);
            if (typeof h === "string") throw sgCdError("DATA1.HDR: " + h);
            await sgCabUnpack(h, cabs, onFile, progress);
        });
        return true;
    }
    return false;
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
        await sgUnzip(file, async (name, bytes, i, n) => {
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

// "Reset graphics settings" on the start page, and ?safe in the address (issue #55): the Enhanced graphics settings in
// settings.json go back to their defaults (Enhanced off, no frame rate cap, v-sync on; "Fix original bugs" is kept), and
// the marker of a failed start goes. Saved games, the extras, keys and every other setting are untouched.
function sgResetGraphics() {
    try {
        const k = sgStoredKey("settings.json");
        if (k !== null) {
            const v = JSON.parse(new TextDecoder().decode(sgUnB64(localStorage.getItem(k))));
            const e = v && v.enhanced;
            if (e && typeof e === "object") {
                v.enhanced = "fix_original_bugs" in e ? { fix_original_bugs: e.fix_original_bugs } : {};
                sgKeepFile("settings.json", new TextEncoder().encode(JSON.stringify(v, null, 2)));
            }
        }
        const t = sgStoredKey(SG_TRIAL_KEY);
        if (t !== null) localStorage.removeItem(t);
    } catch (e) {
        console.warn("could not reset the graphics settings", e);
    }
    sgStatus("The graphics settings were put back to their defaults (Classic, no frame rate cap). Your saved games are kept.");
}

async function sgInit() {
    sgSaveCount();
    const ver = document.getElementById("version");
    if (ver) ver.textContent = "OpenSGolf " + sgVersion();
    // what the last files picked held (said again until other files are picked)
    try {
        const last = JSON.parse(localStorage.getItem(SG_SUMMARY_KEY) || "null");
        if (last && last.text) sgSummary({ text: "Last time: " + last.text, problem: last.problem }, false);
    } catch (e) {}
    if (new URLSearchParams(location.search).has("safe")) sgResetGraphics();
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
            const sum = sgFilesSummary(stored.map(([p]) => p));
            console.log("files: " + sum.text);
            sgSummary(sum, false);
            sgStart();
            // only a problem is said again on a start from the kept files
            if (sum.problem) sgNote(sum.text, true);
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
(function frame() { sgPump(); sgRequestFrame(frame); })();
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
// would open Firefox's menu bar on Windows. The apostrophe (the measuring tape) opens Firefox's quick find, and Ctrl+Shift+C (the Cheats box) the inspector. A key pressed while the page has lost the canvas's focus (after a click on
// the page around it, or back from another window) goes to the game and gives the canvas the focus back. The phone
// keyboard's text box (sgKeyboard) keeps its own keys.
function sgGameKey(e) {
    if (e.metaKey) return false; // Cmd shortcuts stay the browser's (the game needs none)
    if (["PageUp", "PageDown", "Home", "End", "AltLeft", "AltRight", "Quote"].includes(e.code)) return true;
    if (e.ctrlKey && e.shiftKey && e.code === "KeyC") return true; // the hidden Cheats box (Chrome's inspector key)
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
// The game wants the keyboard (a name box is open). iPhone and iPad Safari only raise it for a focus made inside a tap, and
// the game asks a frame after the tap, so a tap on the game while a name box is open focuses the box there (issue #45).
let sgKbdWanted = false;
for (const type of ["touchend", "pointerup"]) {
    window.addEventListener(type, () => {
        if (sgKbdWanted && sgKbd && document.activeElement !== sgKbd) {
            sgKbd.value = " ";
            sgKbd.focus();
        }
    }, true);
}
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
    sgKbdWanted = show;
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
    sgRequestFrame(sgPadPoll);
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
sgRequestFrame(sgPadPoll);
