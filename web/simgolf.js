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

// Folder picking and loading. The game folder is the one that holds Flics, Sounds and Data; the player may pick it or any
// folder above it. Videos (.bik) and Windows binaries are not needed and are skipped.
async function sgLoadFolder(fileList) {
    const status = document.getElementById("status");
    const files = Array.from(fileList);
    let root = null;
    for (const f of files) {
        const parts = f.webkitRelativePath.split("/");
        const i = parts.findIndex(p => p.toLowerCase() === "flics");
        if (i >= 0) {
            root = parts.slice(0, i).join("/");
            break;
        }
    }
    if (root === null) {
        status.textContent = "That folder does not look like a SimGolf game folder (no Flics folder found). Pick the folder that contains Flics, Sounds and Data.";
        return;
    }
    const wanted = files.filter(f => {
        const p = f.webkitRelativePath;
        if (!p.startsWith(root)) return false;
        const low = p.toLowerCase();
        return !(low.endsWith(".bik") || low.endsWith(".exe") || low.endsWith(".dll") || low.endsWith(".lib"));
    });
    let done = 0, bytes = 0;
    for (const f of wanted) {
        const rel = f.webkitRelativePath.substring(root.length).replace(/^\//, "");
        sgAddFile("game/" + rel, new Uint8Array(await f.arrayBuffer()));
        done++;
        bytes += f.size;
        if (done % 50 === 0 || done === wanted.length) {
            status.textContent = "Reading your game files: " + done + " of " + wanted.length + " (" + Math.round(bytes / 1048576) + " MB)";
            await new Promise(r => setTimeout(r, 0));
        }
    }
    sgRestoreSaves();
    document.getElementById("picker").style.display = "none";
    const canvas = document.getElementById("glcanvas");
    canvas.style.display = "block";
    canvas.focus();
    load("simgolf.wasm");
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
window.addEventListener("mousedown", sgStartAudio);
window.addEventListener("keydown", sgStartAudio);
