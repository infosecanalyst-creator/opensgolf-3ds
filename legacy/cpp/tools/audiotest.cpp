// Renders a few sounds through the Mixer to a WAV file, without any audio device: sgaudiotest GAME_DIR out.wav
#include <cstdio>
#include <vector>
#include <SDL.h>
#include "sg/audio.h"

int main(int argc, char** argv) {
    if (argc < 3) { std::fprintf(stderr, "usage: sgaudiotest GAME_DIR out.wav\n"); return 2; }
    SDL_Init(0);
    sg::Mixer m(std::string(argv[1]) + "/Sounds");
    std::printf("%d sound files indexed\n", (int)m.list("").size());
    const char* names[] = {"Golf_Sfx/Drive With Ball.wav", "Golf_Sfx/Ball Drop Fairway.wav", "Golf_Sfx/Ball In Hole.wav", "Applause.wav", "Effects/cash.wav"};
    std::vector<int16_t> out;
    const int step = 44100;  // one second between sounds
    out.resize((size_t)step * 2 * 6);
    int frame = 0;
    for (const char* n : names) {
        int id = m.play(n, 1.0f);
        std::printf("%-34s -> voice %d%s%s\n", n, id, id < 0 ? " ERROR " : "", id < 0 ? m.lastError.c_str() : "");
        m.mix(out.data() + (size_t)frame * 2, step);
        frame += step;
    }
    m.mix(out.data() + (size_t)frame * 2, step);
    // 44-byte WAV header, then PCM
    FILE* f = std::fopen(argv[2], "wb");
    uint32_t dataBytes = (uint32_t)(out.size() * 2), riff = 36 + dataBytes, fmtLen = 16, rate = 44100, byteRate = 44100 * 4;
    uint16_t pcm = 1, ch = 2, align = 4, bits = 16;
    std::fwrite("RIFF", 1, 4, f); std::fwrite(&riff, 4, 1, f); std::fwrite("WAVEfmt ", 1, 8, f); std::fwrite(&fmtLen, 4, 1, f);
    std::fwrite(&pcm, 2, 1, f); std::fwrite(&ch, 2, 1, f); std::fwrite(&rate, 4, 1, f); std::fwrite(&byteRate, 4, 1, f);
    std::fwrite(&align, 2, 1, f); std::fwrite(&bits, 2, 1, f); std::fwrite("data", 1, 4, f); std::fwrite(&dataBytes, 4, 1, f);
    std::fwrite(out.data(), 2, out.size(), f);
    std::fclose(f);
    std::printf("wrote %s\n", argv[2]);
    return 0;
}
