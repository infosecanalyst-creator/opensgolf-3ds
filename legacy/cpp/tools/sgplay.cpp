// Plays the game's Bink videos (Flics/SMSG_IntroFinal.bik, SMSG_ClosingFinal.bik).
//
// The Bink codec is RAD Game Tools' and the original plays it through binkw32.dll. This tool reads the file header itself
// (see parseBinkHeader) but hands the actual video and audio decoding to an installed `ffmpeg` program, which has an open
// implementation of both Bink codecs: `brew install ffmpeg`. ffmpeg runs as two child processes (raw RGB video, raw PCM
// audio) and the audio device clock keeps the picture in step.
//
//   sgplay FILE.bik                       play, Esc or Space to quit
//   sgplay FILE.bik --png out.png --at 12.5   save the frame at that time and exit (no window sound)
//   sgplay FILE.bik --info                print the header
#include <SDL.h>
#include <SDL_opengl.h>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <deque>
#include <mutex>
#include <string>
#include <thread>
#include <vector>
#include "sg/assets.h"
#include <algorithm>

struct BinkHeader {
    char magic[4] = {};          // "BIK" + revision letter (the game's files are BIKi)
    uint32_t fileSize = 0, frames = 0, largestFrame = 0, width = 0, height = 0, fpsNum = 0, fpsDen = 1, flags = 0, audioTracks = 0;
};

static bool parseBinkHeader(const sg::Bytes& d, BinkHeader& h) {
    if (d.size() < 44 || std::memcmp(d.data(), "BIK", 3) != 0) return false;
    auto u32 = [&](size_t o) { return (uint32_t)d[o] | (uint32_t)d[o + 1] << 8 | (uint32_t)d[o + 2] << 16 | (uint32_t)d[o + 3] << 24; };
    std::memcpy(h.magic, d.data(), 4);
    h.fileSize = u32(4) + 8; h.frames = u32(8); h.largestFrame = u32(12);
    h.width = u32(20); h.height = u32(24); h.fpsNum = u32(28); h.fpsDen = u32(32) ? u32(32) : 1; h.flags = u32(36); h.audioTracks = u32(40);
    return h.width > 0 && h.height > 0 && h.width <= 4096 && h.height <= 4096 && h.fpsNum > 0;
}

static std::string shellQuote(const std::string& s) {
    std::string out = "'";
    for (char c : s) { if (c == '\'') out += "'\\''"; else out += c; }
    return out + "'";
}

int main(int argc, char** argv) {
    std::string file, pngOut;
    double at = 0;
    bool info = false;
    for (int i = 1; i < argc; i++) {
        std::string a = argv[i];
        if (a == "--png" && i + 1 < argc) pngOut = argv[++i];
        else if (a == "--at" && i + 1 < argc) at = std::atof(argv[++i]);
        else if (a == "--info") info = true;
        else file = a;
    }
    if (file.empty()) { std::fprintf(stderr, "usage: sgplay FILE.bik [--info] [--png out.png --at SECONDS]\n"); return 2; }
    sg::Bytes raw; BinkHeader h;
    if (!sg::readFile(file, raw) || !parseBinkHeader(raw, h)) { std::fprintf(stderr, "error: cannot read a Bink header from %s\n", file.c_str()); return 1; }
    const double fps = (double)h.fpsNum / h.fpsDen;
    std::printf("%s: %.4s, %ux%u, %u frames at %.3f fps (%.1f s), %u audio track(s)\n", file.c_str(), h.magic, h.width, h.height, h.frames, fps, h.frames / fps, h.audioTracks);
    raw.clear(); raw.shrink_to_fit();
    if (info) return 0;

    if (std::system("ffmpeg -version > /dev/null 2>&1") != 0) {
        std::fprintf(stderr, "error: ffmpeg is not installed (brew install ffmpeg); it decodes the Bink codec\n");
        return 1;
    }
    const size_t frameBytes = (size_t)h.width * h.height * 3;
    const bool still = !pngOut.empty();
    const bool withAudio = !still && h.audioTracks > 0;

    FILE* vp = popen(("ffmpeg -v quiet -i " + shellQuote(file) + " -f rawvideo -pix_fmt rgb24 -").c_str(), "r");
    if (!vp) { std::fprintf(stderr, "error: cannot start ffmpeg\n"); return 1; }

    if (still) {  // decode up to the requested time and save one frame
        const long target = (long)(at * fps);
        std::vector<uint8_t> buf(frameBytes);
        long n = 0;
        while (std::fread(buf.data(), 1, frameBytes, vp) == frameBytes) {
            if (n++ >= target) break;
        }
        pclose(vp);
        sg::Rgba img; img.w = h.width; img.h = h.height; img.px.resize((size_t)h.width * h.height * 4);
        for (size_t i = 0; i < (size_t)h.width * h.height; i++) { img.px[4 * i] = buf[3 * i]; img.px[4 * i + 1] = buf[3 * i + 1]; img.px[4 * i + 2] = buf[3 * i + 2]; img.px[4 * i + 3] = 255; }
        if (!sg::writePng(pngOut, img)) { std::fprintf(stderr, "error: cannot write %s\n", pngOut.c_str()); return 1; }
        std::printf("saved frame %ld to %s\n", n - 1, pngOut.c_str());
        return 0;
    }

    FILE* ap = withAudio ? popen(("ffmpeg -v quiet -i " + shellQuote(file) + " -vn -f s16le -ar 44100 -ac 2 -").c_str(), "r") : nullptr;

    if (SDL_Init(SDL_INIT_VIDEO | (withAudio ? SDL_INIT_AUDIO : 0)) != 0) { std::fprintf(stderr, "SDL_Init: %s\n", SDL_GetError()); return 1; }
    SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);
    SDL_Window* win = SDL_CreateWindow("SimGolf native: video", SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED, (int)h.width, (int)h.height,
                                       SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE | SDL_WINDOW_ALLOW_HIGHDPI);
    if (!win) { std::fprintf(stderr, "SDL_CreateWindow: %s\n", SDL_GetError()); return 1; }
    SDL_GLContext ctx = SDL_GL_CreateContext(win);
    SDL_GL_SetSwapInterval(1);

    // Audio: queue-based device; the clock is the amount of audio that has left the queue.
    SDL_AudioDeviceID dev = 0;
    if (withAudio) {
        SDL_AudioSpec want; SDL_zero(want);
        want.freq = 44100; want.format = AUDIO_S16SYS; want.channels = 2; want.samples = 2048;
        dev = SDL_OpenAudioDevice(nullptr, 0, &want, nullptr, 0);
        if (!dev) std::fprintf(stderr, "sound: no audio device, playing silent video\n");
    }
    std::atomic<bool> quit{false};
    std::atomic<uint64_t> audioQueued{0};
    std::thread audioThread;
    if (dev && ap) {
        SDL_PauseAudioDevice(dev, 0);
        audioThread = std::thread([&] {
            std::vector<uint8_t> buf(44100 * 4 / 10);   // 100 ms chunks
            while (!quit) {
                if (SDL_GetQueuedAudioSize(dev) > 44100u * 4u / 2u) { SDL_Delay(10); continue; }
                size_t n = std::fread(buf.data(), 1, buf.size(), ap);
                if (n == 0) break;
                SDL_QueueAudio(dev, buf.data(), (Uint32)n);
                audioQueued += n;
            }
        });
    }

    // Video frames are read on a thread into a small queue.
    std::mutex m; std::condition_variable cv;
    std::deque<std::vector<uint8_t>> queue;
    bool videoDone = false;
    std::thread videoThread([&] {
        while (!quit) {
            std::vector<uint8_t> f(frameBytes);
            if (std::fread(f.data(), 1, frameBytes, vp) != frameBytes) break;
            std::unique_lock<std::mutex> lock(m);
            cv.wait(lock, [&] { return queue.size() < 6 || quit; });
            queue.push_back(std::move(f));
        }
        std::lock_guard<std::mutex> lock(m);
        videoDone = true;
    });

    GLuint tex = 0;
    glGenTextures(1, &tex);
    glBindTexture(GL_TEXTURE_2D, tex);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glPixelStorei(GL_UNPACK_ALIGNMENT, 1);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGB, (GLsizei)h.width, (GLsizei)h.height, 0, GL_RGB, GL_UNSIGNED_BYTE, nullptr);

    const Uint32 start = SDL_GetTicks();
    long shown = -1;
    bool haveFrame = false;
    std::vector<uint8_t> current;
    long nextFrame = 0;   // index of the frame at the head of the queue
    while (!quit) {
        SDL_Event e;
        while (SDL_PollEvent(&e)) {
            if (e.type == SDL_QUIT || (e.type == SDL_KEYDOWN && (e.key.keysym.sym == SDLK_ESCAPE || e.key.keysym.sym == SDLK_SPACE))) quit = true;
        }
        double clock = (dev && ap) ? (double)((double)audioQueued - (double)SDL_GetQueuedAudioSize(dev)) / (44100.0 * 4.0) : (SDL_GetTicks() - start) / 1000.0;
        // Take every frame that is due; show the newest of them.
        bool finished = false;
        {
            std::lock_guard<std::mutex> lock(m);
            while (!queue.empty() && nextFrame / fps <= clock) {
                current = std::move(queue.front()); queue.pop_front();
                shown = nextFrame++; haveFrame = true;
            }
            finished = videoDone && queue.empty();
        }
        cv.notify_all();
        if (haveFrame && current.size() == frameBytes) {
            glBindTexture(GL_TEXTURE_2D, tex);
            glTexSubImage2D(GL_TEXTURE_2D, 0, 0, 0, (GLsizei)h.width, (GLsizei)h.height, GL_RGB, GL_UNSIGNED_BYTE, current.data());
            current.clear();
        }
        int dw, dh; SDL_GL_GetDrawableSize(win, &dw, &dh);
        glViewport(0, 0, dw, dh);
        glClearColor(0, 0, 0, 1); glClear(GL_COLOR_BUFFER_BIT);
        if (shown >= 0) {
            // Letterbox to keep the aspect ratio.
            float sx = 1, sy = 1, ar = (float)h.width / h.height, wr = (float)dw / std::max(1, dh);
            if (wr > ar) sx = ar / wr; else sy = wr / ar;
            glMatrixMode(GL_PROJECTION); glLoadIdentity(); glMatrixMode(GL_MODELVIEW); glLoadIdentity();
            glEnable(GL_TEXTURE_2D); glBindTexture(GL_TEXTURE_2D, tex); glColor3f(1, 1, 1);
            glBegin(GL_QUADS);
            glTexCoord2f(0, 1); glVertex2f(-sx, -sy); glTexCoord2f(1, 1); glVertex2f(sx, -sy);
            glTexCoord2f(1, 0); glVertex2f(sx, sy); glTexCoord2f(0, 0); glVertex2f(-sx, sy);
            glEnd();
        }
        SDL_GL_SwapWindow(win);
        if (finished) { SDL_Delay(300); break; }
        SDL_Delay(2);
    }
    quit = true;
    cv.notify_all();
    // Drain the pipes while the reader threads notice `quit`, then close them (ffmpeg stops when its pipe closes).
    if (videoThread.joinable()) videoThread.join();
    if (audioThread.joinable()) audioThread.join();
    if (vp) pclose(vp);
    if (ap) pclose(ap);
    if (dev) SDL_CloseAudioDevice(dev);
    SDL_GL_DeleteContext(ctx);
    SDL_DestroyWindow(win);
    SDL_Quit();
    return 0;
}
