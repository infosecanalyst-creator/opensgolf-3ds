// Small deterministic random source shared by the social modules (membership, visitors). Not the original's generator.
#pragma once
#include <cstdint>
namespace sg {
struct SocialRng {
    uint64_t s = 0x9e3779b97f4a7c15ull;
    explicit SocialRng(uint64_t seed = 1) { s ^= seed * 0xbf58476d1ce4e5b9ull; if (!s) s = 1; }
    uint32_t next() { s ^= s << 13; s ^= s >> 7; s ^= s << 17; return uint32_t(s >> 16); }
    int below(int n) { return n <= 0 ? 0 : int(next() % uint32_t(n)); }   // 0..n-1
};
}
