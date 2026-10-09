// Ball flight, bounce and roll with the per-tick rules read from the publisher's golf.exe (docs/DECODE_PLAYCORE.md sections 1 and 3, EXACT unless marked).
// Units are the exe's: 1024 per tile, speed in 1/16 unit per tick, vertical speed in 1/32 unit per tick, 64 gravity per tick.
// PLACEHOLDERS: the slope term of the friction, the minimum friction on tile edges, the sideways curve of draws and fades (not applied), and the tick rate in seconds.
#pragma once
#include <algorithm>
#include <cmath>
#include <vector>
#include "sg/lie.h"

namespace sg {
namespace ballphys {

struct Pt { float x, z, h; };   // exe units; x, z relative to the strike point

struct Run {
    std::vector<Pt> pts;          // one point per tick, starting at the strike
    int firstLand = -1;           // index of the first ground contact
    int landTile = -1, restTile = -1;   // port tile types under the first contact and under the resting ball, -1 off the map
    bool water = false, oob = false;    // the ball stopped in water, or left the map
    int hitTick = -1, hitType = -1;     // a tree or building struck in flight
    float restX = 0, restZ = 0;
    int bounces = 0;
};

// tileAt(x, z): port tile type under the point (exe units relative to the strike), -1 off the map. rnd(): 0..1.
// theme picks the tree height bands (docs/DECODE_EVENTS_SHOTS.md 3.1). obstacles=false is the preview the exe uses when it picks a club.
template <class TileAt, class Rnd>
Run run(float dirx, float dirz, int speed, int vert, int theme, bool obstacles, TileAt tileAt, Rnd rnd) {
    Run r; float x = 0, z = 0, h = 0; bool hit = false; int tick = 0;
    r.pts.push_back({0, 0, 0});
    while (tick < 3000) {
        ++tick;
        x += dirx * (float)speed / 16.0f; z += dirz * (float)speed / 16.0f; h += (float)vert / 32.0f;
        const bool air = h > 0 || vert > 0;
        vert -= 64;
        if (air) speed -= speed >> 5;
        const int here = tileAt(x, z);
        if (here < 0) { h = std::max(h, 0.0f); r.oob = true; r.pts.push_back({x, z, h}); if (r.firstLand < 0) { r.firstLand = (int)r.pts.size() - 1; r.landTile = -1; } r.restTile = -1; break; }
        if (obstacles && !hit && h >= 2.0f && (here == TT_Woods || here == TT_Building)) {
            int lo = 0, hi = 200;
            if (here == TT_Woods) { const int r100 = (int)(rnd() * 100.0f); lo = theme == 2 || theme == 3 ? 20 : 50; hi = (theme == 2 ? 100 : 400) + r100; }
            if (h > lo && h < hi) {
                const float fx = x / 1024.0f - std::floor(x / 1024.0f) - 0.5f, fz = z / 1024.0f - std::floor(z / 1024.0f) - 0.5f;
                if (std::hypot(fx, fz) * 1024.0f < rnd() * 384.0f) {
                    hit = true; r.hitTick = (int)r.pts.size(); r.hitType = here;
                    const float turn = (64.0f + rnd() * 128.0f) / 256.0f * 6.2831853f, c = std::cos(turn), s = std::sin(turn);
                    const float nx = dirx * c - dirz * s, nz = dirx * s + dirz * c; dirx = nx; dirz = nz;
                    speed = (int)((float)speed * (1.0f - rnd())); vert = std::min(vert, 0);   // PLACEHOLDER: the exe's ball drops after a hit
                }
            }
        }
        if (h <= 0 && vert < 0) {   // ground contact
            h = 0;
            const Lie& L = lieOf(here);
            if (r.firstLand < 0) { r.firstLand = (int)r.pts.size(); r.landTile = here; }
            if (L.cls == 17) { r.water = true; r.pts.push_back({x, z, 0}); r.restTile = here; break; }
            int rebound = std::clamp(-64 + (-vert) * (int)L.bounce / 12, 0, 9999);
            if (rebound < 0x80) { rebound = 0; } else r.bounces++;
            vert = rebound;
        }
        if (h == 0 && vert == 0) {   // rolling
            const Lie& L = lieOf(here);
            int f = std::clamp((int)L.friction, 0, 99);
            if (tick > 128) f = std::min(f, 4);
            if (f < 5) speed -= (speed >> f) / 2; else speed = speed - (speed >> 6) + 32;
        }
        r.pts.push_back({x, z, h});
        if (speed < 0x40 && h == 0 && vert == 0) break;
    }
    r.restX = x; r.restZ = z;
    if (r.restTile < 0 && !r.oob) r.restTile = tileAt(x, z);
    return r;
}

}  // namespace ballphys
}  // namespace sg
