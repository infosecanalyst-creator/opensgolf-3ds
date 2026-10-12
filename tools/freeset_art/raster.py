"""The free set's offline sprite renderer: triangles with a flat colour each, seen through the game's camera (orthographic,
pitched 38.682 degrees, turned 45 degrees for view 0, docs/TERRAIN.md and docs/SPRITES.md), lit by a light fixed to the
camera, rasterised with a depth buffer at a few samples per pixel. Also casts the ground shadow the game draws under a
sprite (falling right and away from the viewer, like the light the game's shadow sprites assume)."""
import numpy as np

PITCH = 38.682
UNITS_PER_PX = 1767.0 / 800.0
# towards the light, in eye space (x right, y up, z towards the viewer): from the upper left, a little in front
LIGHT = np.array([-0.55, 0.62, 0.56])
LIGHT = LIGHT / np.linalg.norm(LIGHT)
# a ground point at height h (world units) casts its shadow this many world units across the screen (right) and up the
# screen per unit of height (measured from the shape of the original's tree shadows: about 0.7 right and 0.55 up)
SHADOW_DX = 0.68
SHADOW_DY = 0.55


def rot_y(deg):
    a = np.radians(deg)
    c, s = np.cos(a), np.sin(a)
    return np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])


def rot_x(deg):
    a = np.radians(deg)
    c, s = np.cos(a), np.sin(a)
    return np.array([[1, 0, 0], [0, c, -s], [0, s, c]])


def eye_matrix(yaw):
    """World to eye: the game turns the world by 45 + yaw about Y, then pitches it."""
    return rot_x(PITCH) @ rot_y(45.0 + yaw)


class Frame:
    """One sprite frame being drawn: w x h pixels with the ground point at (ax, ay), `ss` samples per pixel each way."""

    def __init__(self, w, h, ax, ay, ss=4):
        self.w, self.h, self.ax, self.ay, self.ss = w, h, ax, ay, ss
        W, H = w * ss, h * ss
        self.col = np.zeros((H, W, 3))
        self.depth = np.full((H, W), -np.inf)
        self.tag = np.full((H, W), -1, dtype=np.int32)
        self.shade = np.zeros((H, W))
        self.shadow = np.zeros((H, W), dtype=bool)

    def to_px(self, eye):
        """Eye-space points (world units) to sample coordinates."""
        s = self.ss / UNITS_PER_PX
        return np.stack([(self.ax + 0.0) * self.ss + eye[..., 0] * s, self.ay * self.ss - eye[..., 1] * s], -1)

    def draw(self, tris_world, cols, yaw=0.0, tags=None, ambient=0.5, diffuse=0.62, two_sided=True, emissive=None):
        """Draws triangles (n x 3 x 3 world units, ground at y = 0, the object's ground point at the origin) with colours
        (n x 3, 0..1). `tags` (n ints) are kept per sample with the brightness (for palette slots)."""
        if len(tris_world) == 0:
            return
        m = eye_matrix(yaw)
        eye = tris_world @ m.T
        nrm = np.cross(eye[:, 1] - eye[:, 0], eye[:, 2] - eye[:, 0])
        ln = np.linalg.norm(nrm, axis=1, keepdims=True)
        ok = ln[:, 0] > 1e-12
        nrm = nrm / np.maximum(ln, 1e-12)
        if two_sided:
            nrm = np.where(nrm[:, 2:3] < 0, -nrm, nrm)
        lam = np.clip(nrm @ LIGHT, 0, 1)
        # sky light from above, a little bounce from below
        sky = 0.5 + 0.5 * nrm[:, 1]
        bright = ambient * (0.75 + 0.25 * sky) + diffuse * lam
        if emissive is not None:
            bright = np.where(emissive, 1.0, bright)
        px = self.to_px(eye)
        z = eye[..., 2]
        tg = np.full(len(px), -1, dtype=np.int32) if tags is None else np.asarray(tags, dtype=np.int32)
        col = np.clip(cols * bright[:, None], 0, 1)
        self._raster(px[ok], z[ok], col[ok], bright[ok], tg[ok])

    def _raster(self, px, z, col, bright, tg):
        """Scan-converts triangles (sample coordinates px n x 3 x 2, depth z n x 3) with a depth test, in batches of
        triangles of similar size."""
        H, W = self.depth.shape
        lo = np.floor(px.min(1)).astype(int)
        hi = np.ceil(px.max(1)).astype(int)
        size = np.maximum(hi[:, 0] - lo[:, 0], hi[:, 1] - lo[:, 1]) + 1
        (ax_, ay_), (bx, by), (cx, cy) = px[:, 0].T, px[:, 1].T, px[:, 2].T
        den = (by - cy) * (ax_ - cx) + (cx - bx) * (ay_ - cy)
        good = np.abs(den) > 1e-9
        for S in (4, 8, 16, 32, 64, 128, 256, 100000):
            sel = np.nonzero(good & (size <= S))[0]
            good[sel] = False
            if not len(sel):
                continue
            # big triangles one at a time on their own box
            for chunk in (np.array_split(sel, max(1, len(sel) * S * S // 400000 + 1)) if S <= 256 else [[i] for i in sel]):
                chunk = np.asarray(chunk)
                if S > 256:
                    i = chunk[0]
                    Sx = hi[i, 0] - lo[i, 0] + 1
                    Sy = hi[i, 1] - lo[i, 1] + 1
                    oy, ox = np.mgrid[0:Sy, 0:Sx]
                else:
                    oy, ox = np.mgrid[0:S, 0:S]
                gx = lo[chunk, 0][:, None, None] + ox[None] + 0.5
                gy = lo[chunk, 1][:, None, None] + oy[None] + 0.5
                A = lambda v: v[chunk][:, None, None]
                l1 = ((A(by) - A(cy)) * (gx - A(cx)) + (A(cx) - A(bx)) * (gy - A(cy))) / A(den)
                l2 = ((A(cy) - A(ay_)) * (gx - A(cx)) + (A(ax_) - A(cx)) * (gy - A(cy))) / A(den)
                l3 = 1 - l1 - l2
                inside = (l1 >= -1e-6) & (l2 >= -1e-6) & (l3 >= -1e-6) & (gx >= 0) & (gy >= 0) & (gx < W) & (gy < H)
                if not inside.any():
                    continue
                zz = l1 * z[chunk, 0][:, None, None] + l2 * z[chunk, 1][:, None, None] + l3 * z[chunk, 2][:, None, None]
                ti = np.broadcast_to(chunk[:, None, None], inside.shape)[inside]
                pix = (gy[inside].astype(np.int64) * W + gx[inside].astype(np.int64))
                zv = zz[inside]
                # the nearest candidate per sample, then the depth test against what is there
                order = np.lexsort((zv, pix))
                pix, zv, ti = pix[order], zv[order], ti[order]
                last = np.r_[pix[1:] != pix[:-1], True]
                pix, zv, ti = pix[last], zv[last], ti[last]
                d = self.depth.reshape(-1)
                win = zv > d[pix]
                pix, zv, ti = pix[win], zv[win], ti[win]
                d[pix] = zv
                self.col.reshape(-1, 3)[pix] = col[ti]
                self.shade.reshape(-1)[pix] = bright[ti]
                self.tag.reshape(-1)[pix] = tg[ti]

    def draw_balls(self, centres, radii, cols, yaw=0.0, mass=None, leafy=0.35, rag=0.18, seed=0, tags=None,
                   ambient=0.5, diffuse=0.72):
        """Draws spheres (n x 3 world units centres, n radii, n x 3 colours) as lit balls with leafy, ragged outlines: the
        foliage of trees and bushes. `mass` (centre, radius) darkens the inside of a crown (light reaches its outside)."""
        if len(centres) == 0:
            return
        m = eye_matrix(yaw)
        eye = np.asarray(centres) @ m.T
        pc = self.to_px(eye)
        s = self.ss / UNITS_PER_PX
        H, W = self.depth.shape
        rng = np.random.default_rng(seed)
        if mass is not None:
            mc = np.asarray(mass[0]) @ m.T
        for i in range(len(centres)):
            r = radii[i] * s
            cx, cy = pc[i]
            x0, x1 = max(int(cx - r * 1.3), 0), min(int(cx + r * 1.3) + 1, W - 1)
            y0, y1 = max(int(cy - r * 1.3), 0), min(int(cy + r * 1.3) + 1, H - 1)
            if x0 > x1 or y0 > y1:
                continue
            gx, gy = np.meshgrid(np.arange(x0, x1 + 1) + 0.5, np.arange(y0, y1 + 1) + 0.5)
            dx = (gx - cx) / r
            dy = -(gy - cy) / r
            ang = np.arctan2(dy, dx)
            ph = rng.uniform(0, 6.3, 3)
            edge = 1 + rag * (0.55 * np.sin(ang * 5 + ph[0]) + 0.3 * np.sin(ang * 9 + ph[1]) + 0.25 * np.sin(ang * 14 + ph[2]))
            d2 = dx * dx + dy * dy
            inside = d2 < edge * edge
            if not inside.any():
                continue
            nz = np.sqrt(np.clip(1 - d2 / (edge * edge), 0, 1))
            n = np.stack([dx, dy, nz], -1)
            # leaf clusters: a grain of random tilts over the ball, coarser than a pixel
            if leafy > 0:
                kx = np.floor(gx / max(self.ss * 1.3, 1)).astype(np.int64)
                ky = np.floor(gy / max(self.ss * 1.3, 1)).astype(np.int64)
                h = (kx * 73856093) ^ (ky * 19349663) ^ (i * 83492791 + seed)
                jx = ((h & 1023) / 1023.0 - 0.5)
                jy = (((h >> 10) & 1023) / 1023.0 - 0.5)
                n = n + leafy * np.stack([jx, jy, np.zeros_like(jx)], -1) * 2
            n = n / np.linalg.norm(n, axis=-1, keepdims=True)
            lam = np.clip(n @ LIGHT, 0, 1)
            sky = 0.5 + 0.5 * n[..., 1]
            bright = ambient * (0.7 + 0.3 * sky) + diffuse * lam
            z = eye[i, 2] + nz * radii[i]
            if mass is not None:
                # the point on the ball, in eye space, against the crown's centre: inner leaves are in shade
                p = np.stack([eye[i, 0] + dx * radii[i], eye[i, 1] + dy * radii[i], z], -1)
                rel = np.linalg.norm(p - mc, axis=-1) / mass[1]
                bright = bright * np.clip(0.55 + 0.5 * rel, 0.55, 1.08)
            sub = self.depth[y0:y1 + 1, x0:x1 + 1]
            win = inside & (z > sub)
            if not win.any():
                continue
            sub[win] = z[win]
            c = np.clip(np.asarray(cols[i])[None, None, :] * bright[..., None], 0, 1)
            self.col[y0:y1 + 1, x0:x1 + 1][win] = c[win]
            self.shade[y0:y1 + 1, x0:x1 + 1][win] = bright[win]
            self.tag[y0:y1 + 1, x0:x1 + 1][win] = -1 if tags is None else tags[i]

    def cast_balls(self, centres, radii, yaw=0.0):
        """Adds the ground shadow of spheres (as ellipses) to the shadow mask."""
        if len(centres) == 0:
            return
        m = eye_matrix(yaw)
        c = np.asarray(centres, float)
        h = c[:, 1].copy()
        foot = c.copy()
        foot[:, 1] = 0
        eye = foot @ m.T
        eye[:, 0] += h * SHADOW_DX
        eye[:, 1] += h * SHADOW_DY
        pc = self.to_px(eye)
        s = self.ss / UNITS_PER_PX
        H, W = self.shadow.shape
        for i in range(len(c)):
            rx = radii[i] * s * 1.05
            ry = rx * 0.85
            cx, cy = pc[i]
            x0, x1 = max(int(cx - rx), 0), min(int(cx + rx) + 1, W - 1)
            y0, y1 = max(int(cy - ry), 0), min(int(cy + ry) + 1, H - 1)
            if x0 > x1 or y0 > y1:
                continue
            gx, gy = np.meshgrid(np.arange(x0, x1 + 1) + 0.5, np.arange(y0, y1 + 1) + 0.5)
            self.shadow[y0:y1 + 1, x0:x1 + 1] |= ((gx - cx) / rx) ** 2 + ((gy - cy) / ry) ** 2 < 1

    def cast(self, tris_world, yaw=0.0):
        """Adds the ground shadow of the triangles (world units, y up) to the shadow mask."""
        if len(tris_world) == 0:
            return
        m = eye_matrix(yaw)
        h = tris_world[..., 1].copy()
        foot = tris_world.copy()
        foot[..., 1] = 0
        eye = foot @ m.T
        eye[..., 0] += h * SHADOW_DX
        eye[..., 1] += h * SHADOW_DY
        px = self.to_px(eye)
        # draw the shadow triangles into a scratch depth layer and take their coverage
        keep = (self.depth, self.col, self.shade, self.tag)
        H, W = self.shadow.shape
        self.depth = np.full((H, W), -np.inf)
        self.col = np.zeros((H, W, 3))
        self.shade = np.zeros((H, W))
        self.tag = np.zeros((H, W), dtype=np.int32)
        n = len(px)
        self._raster(px, np.zeros((n, 3)), np.zeros((n, 3)), np.zeros(n), np.zeros(n, dtype=np.int32))
        self.shadow |= np.isfinite(self.depth)
        self.depth, self.col, self.shade, self.tag = keep

    def resolve(self, cut=0.45, edge_dark=0.12):
        """The frame: (rgb h x w x 3 in 0..1, opaque mask, tag per pixel, brightness per pixel)."""
        ss = self.ss
        H, W = self.h, self.w
        cov = np.isfinite(self.depth).reshape(H, ss, W, ss)
        n = cov.sum((1, 3))
        alpha = n / (ss * ss)
        col = (self.col.reshape(H, ss, W, ss, 3) * cov[..., None]).sum((1, 3)) / np.maximum(n, 1)[..., None]
        opaque = alpha >= cut
        # the tag of a pixel: the most common among its covered samples
        n2 = ss * ss
        tags = self.tag.reshape(H, ss, W, ss).transpose(0, 2, 1, 3).reshape(H, W, n2)
        shade = self.shade.reshape(H, ss, W, ss).transpose(0, 2, 1, 3).reshape(H, W, n2)
        covf = np.isfinite(self.depth).reshape(H, ss, W, ss).transpose(0, 2, 1, 3).reshape(H, W, n2)
        tagc = np.where(covf, tags, -999)
        counts = np.zeros((H, W, n2), dtype=np.int32)
        for k in range(n2):
            counts[..., k] = ((tagc == tagc[..., k:k + 1]) & covf).sum(-1) * covf[..., k]
        best = np.take_along_axis(tagc, counts.argmax(-1)[..., None], -1)[..., 0]
        same = (tagc == best[..., None]) & covf
        bright = (shade * same).sum(-1) / np.maximum(same.sum(-1), 1)
        tag = np.where(opaque, best, -1).astype(np.int32)
        bright = np.where(opaque, bright, 0)
        # darken the outermost pixels a touch so the sprite reads against the grass
        if edge_dark > 0:
            pad = np.pad(opaque, 1)
            inner = pad[:-2, 1:-1] & pad[2:, 1:-1] & pad[1:-1, :-2] & pad[1:-1, 2:]
            rim = opaque & ~inner
            col[rim] *= 1 - edge_dark
            bright[rim] *= 1 - edge_dark
        shadow = self.shadow.reshape(H, ss, W, ss).mean((1, 3)) >= 0.5
        return col, opaque, tag, bright, shadow
