// SimGolf native port: sound. All of the game's sounds are plain PCM .wav files under Sounds/, so there is nothing to
// decode beyond SDL's WAV loader; this is a small mixer (clips converted to 44.1 kHz stereo, a handful of voices, looping)
// plus an SDL output device. The mixing core has no SDL device dependency, so it can render to a buffer for tests.
#pragma once
#include <cstdint>
#include <map>
#include <memory>
#include <mutex>
#include <string>
#include <vector>

namespace sg {

class Mixer {
  public:
    explicit Mixer(const std::string& soundsDir);
    // Finds a clip by path relative to Sounds/ (case-insensitive, forward slashes) and plays it. Returns a voice id, or -1.
    int play(const std::string& rel, float volume = 1.0f, bool loop = false);
    // Positioned play (the exe's FUN_0040c500 path): pan -64..63 (left to right), pitch in cents (+-1200), start delay in milliseconds.
    int playAt(const std::string& rel, float volume, int pan, int pitchCents, int delayMs);
    void stop(int voice);
    void stopAll();
    void setVoiceVolume(int voice, float volume);
    void mix(int16_t* out, int frames);    // interleaved stereo S16 at 44100 Hz; thread safe
    float master = 0.8f;
    int clipsLoaded() const { return (int)clips_.size(); }
    void addFolder(const std::string& realDir, const std::string& keyPrefix);   // indexes extra wav files under keyPrefix (for example the voice folders that sit beside Sounds/)
    bool known(const std::string& rel) const;
    std::vector<std::string> list(const std::string& folderPrefix) const;   // relative paths under a folder, sorted
    std::string lastError;

  private:
    struct Clip { std::vector<int16_t> pcm; };  // stereo interleaved
    struct Voice { int id; const Clip* clip; size_t pos; float vol; bool loop; bool general = false; double fpos = 0, rate = 1; float gl = 1, gr = 1; long delay = 0; };
    std::string dir_;
    std::map<std::string, std::string> index_;  // lower-case relative path -> real path
    std::map<std::string, std::unique_ptr<Clip>> clips_;
    std::vector<Voice> voices_;
    int nextId_ = 1;
    std::mutex m_;
    const Clip* load(const std::string& lowerRel);
};

// Opens the default output device and feeds it from a Mixer. Returns false (and stays silent) when there is no device.
class AudioDevice {
  public:
    AudioDevice() = default;
    ~AudioDevice();
    bool open(Mixer& mixer);
    void close();
    bool isOpen() const { return dev_ != 0; }
  private:
    unsigned dev_ = 0;
};

}  // namespace sg
