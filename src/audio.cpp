#include "sg/audio.h"
#include <SDL.h>
#include <algorithm>
#include <cstring>
#include <filesystem>

namespace sg {

static std::string lowerStr(std::string s) {
    for (char& c : s) c = (char)std::tolower((unsigned char)c);
    return s;
}

Mixer::Mixer(const std::string& soundsDir) : dir_(soundsDir) {
    namespace fs = std::filesystem;
    std::error_code ec;
    for (auto it = fs::recursive_directory_iterator(soundsDir, ec); !ec && it != fs::recursive_directory_iterator(); it.increment(ec)) {
        if (!it->is_regular_file()) continue;
        std::string rel = fs::relative(it->path(), soundsDir, ec).generic_string();
        std::string low = lowerStr(rel);
        if (low.size() > 4 && low.compare(low.size() - 4, 4, ".wav") == 0) index_[low] = it->path().string();
    }
    if (index_.empty()) lastError = "no sounds found in " + soundsDir;
}

void Mixer::addFolder(const std::string& realDir, const std::string& keyPrefix) {
    namespace fs = std::filesystem;
    std::error_code ec;
    std::lock_guard<std::mutex> lk(m_);
    for (auto it = fs::recursive_directory_iterator(realDir, ec); !ec && it != fs::recursive_directory_iterator(); it.increment(ec)) {
        if (!it->is_regular_file()) continue;
        std::string low = lowerStr(keyPrefix + fs::relative(it->path(), realDir, ec).generic_string());
        if (low.size() > 4 && low.compare(low.size() - 4, 4, ".wav") == 0) index_[low] = it->path().string();
    }
}

bool Mixer::known(const std::string& rel) const { return index_.count(lowerStr(rel)) != 0; }

std::vector<std::string> Mixer::list(const std::string& prefix) const {
    std::vector<std::string> out;
    const std::string p = lowerStr(prefix);
    for (const auto& kv : index_) if (kv.first.compare(0, p.size(), p) == 0) out.push_back(kv.first);
    return out;
}

const Mixer::Clip* Mixer::load(const std::string& lowerRel) {
    auto c = clips_.find(lowerRel);
    if (c != clips_.end()) return c->second.get();
    auto f = index_.find(lowerRel);
    if (f == index_.end()) { lastError = "unknown sound " + lowerRel; return nullptr; }
    SDL_AudioSpec spec;
    Uint8* buf = nullptr;
    Uint32 len = 0;
    if (!SDL_LoadWAV(f->second.c_str(), &spec, &buf, &len)) { lastError = std::string("cannot load ") + f->second + ": " + SDL_GetError(); return nullptr; }
    SDL_AudioCVT cvt;
    int r = SDL_BuildAudioCVT(&cvt, spec.format, spec.channels, spec.freq, AUDIO_S16SYS, 2, 44100);
    auto clip = std::make_unique<Clip>();
    if (r < 0) { SDL_FreeWAV(buf); lastError = "cannot convert " + f->second; return nullptr; }
    if (r == 0) {
        clip->pcm.resize(len / 2);
        std::memcpy(clip->pcm.data(), buf, len & ~1u);
    } else {
        cvt.len = (int)len;
        std::vector<Uint8> work((size_t)len * (size_t)cvt.len_mult);
        std::memcpy(work.data(), buf, len);
        cvt.buf = work.data();
        SDL_ConvertAudio(&cvt);
        clip->pcm.resize((size_t)cvt.len_cvt / 2);
        std::memcpy(clip->pcm.data(), work.data(), (size_t)cvt.len_cvt & ~1u);
    }
    SDL_FreeWAV(buf);
    const Clip* ptr = clip.get();
    clips_[lowerRel] = std::move(clip);
    return ptr;
}

int Mixer::play(const std::string& rel, float volume, bool loop) {
    std::lock_guard<std::mutex> lock(m_);
    const Clip* c = load(lowerStr(rel));
    if (!c || c->pcm.empty()) return -1;
    if (voices_.size() >= 32) voices_.erase(voices_.begin());   // steal the oldest
    int id = nextId_++;
    voices_.push_back({id, c, 0, volume, loop});
    return id;
}

void Mixer::stop(int voice) {
    std::lock_guard<std::mutex> lock(m_);
    voices_.erase(std::remove_if(voices_.begin(), voices_.end(), [&](const Voice& v) { return v.id == voice; }), voices_.end());
}
void Mixer::stopAll() { std::lock_guard<std::mutex> lock(m_); voices_.clear(); }
void Mixer::setVoiceVolume(int voice, float volume) {
    std::lock_guard<std::mutex> lock(m_);
    for (Voice& v : voices_) if (v.id == voice) v.vol = volume;
}

void Mixer::mix(int16_t* out, int frames) {
    std::lock_guard<std::mutex> lock(m_);
    std::vector<int32_t> acc((size_t)frames * 2, 0);
    for (Voice& v : voices_) {
        const size_t total = v.clip->pcm.size();
        const int gain = (int)(v.vol * master * 256.0f);
        size_t i = 0;
        while (i < acc.size()) {
            if (v.pos >= total) { if (v.loop) v.pos = 0; else break; }
            size_t n = std::min(acc.size() - i, total - v.pos);
            for (size_t k = 0; k < n; k++) acc[i + k] += (v.clip->pcm[v.pos + k] * gain) >> 8;
            v.pos += n; i += n;
        }
    }
    voices_.erase(std::remove_if(voices_.begin(), voices_.end(), [](const Voice& v) { return !v.loop && v.pos >= v.clip->pcm.size(); }), voices_.end());
    for (size_t i = 0; i < acc.size(); i++) out[i] = (int16_t)std::clamp(acc[i], -32768, 32767);
}

static void SDLCALL audioCallback(void* user, Uint8* stream, int len) {
    static_cast<Mixer*>(user)->mix(reinterpret_cast<int16_t*>(stream), len / 4);
}

AudioDevice::~AudioDevice() { close(); }

bool AudioDevice::open(Mixer& mixer) {
    if (SDL_InitSubSystem(SDL_INIT_AUDIO) != 0) return false;
    SDL_AudioSpec want, have;
    SDL_zero(want);
    want.freq = 44100; want.format = AUDIO_S16SYS; want.channels = 2; want.samples = 1024;
    want.callback = audioCallback; want.userdata = &mixer;
    SDL_AudioDeviceID d = SDL_OpenAudioDevice(nullptr, 0, &want, &have, 0);
    if (!d) return false;
    if (have.freq != 44100 || have.channels != 2 || have.format != AUDIO_S16SYS) { SDL_CloseAudioDevice(d); return false; }
    dev_ = d;
    SDL_PauseAudioDevice(d, 0);
    return true;
}

void AudioDevice::close() {
    if (dev_) { SDL_CloseAudioDevice(dev_); dev_ = 0; }
}

}  // namespace sg
