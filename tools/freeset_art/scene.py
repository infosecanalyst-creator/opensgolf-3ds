"""Scenes for the sprite renderer: triangles and leafy balls with colours (and palette tags for people), helpers to build
simple solids, and fitting a scene into a sprite frame at the frame's ground point."""
import numpy as np

from raster import SHADOW_DX, SHADOW_DY, UNITS_PER_PX, Frame, eye_matrix


class Scene:
    def __init__(self):
        self.tris = []
        self.cols = []
        self.ttags = []
        self.balls = []  # (centre, radius, colour, tag)
        self.mass = None
        self.leafy = 0.35
        self.rag = 0.18
        self.seed = 0
        self.no_shadow = False
        self.emissive = []
        self.pal = {}
        self.ambient = 0.5
        self.diffuse = 0.62
        # ramp models: take a tone from each triangle's colour (its mean over 0.5), as the ramp ignores the colour
        self.tone = False

    def add_tris(self, t, col, tag=-1, emissive=False):
        t = np.asarray(t, float).reshape(-1, 3, 3)
        col = np.asarray(col, float)
        if col.ndim == 1:
            col = np.tile(col, (len(t), 1))
        self.tris.append(t)
        self.cols.append(col)
        self.ttags.append(np.full(len(t), tag, dtype=np.int32))
        self.emissive.append(np.full(len(t), emissive))

    def add_ball(self, c, r, col, tag=-1):
        self.balls.append((np.asarray(c, float), float(r), np.asarray(col, float), tag))

    def merged(self):
        if self.tris:
            return np.concatenate(self.tris), np.concatenate(self.cols), np.concatenate(self.ttags), np.concatenate(self.emissive)
        return np.zeros((0, 3, 3)), np.zeros((0, 3)), np.zeros(0, np.int32), np.zeros(0, bool)

    def transformed(self, s=1.0, rot=None, move=(0, 0, 0)):
        """A copy scaled by s, turned by the 3x3 matrix rot and moved."""
        out = Scene()
        out.leafy, out.rag, out.seed, out.no_shadow = self.leafy, self.rag, self.seed, self.no_shadow
        out.pal = self.pal
        out.ambient, out.diffuse = self.ambient, self.diffuse
        out.tone = self.tone
        r = np.eye(3) if rot is None else rot
        mv = np.asarray(move, float)
        for t, c, g, e in zip(self.tris, self.cols, self.ttags, self.emissive):
            out.tris.append(t * s @ r.T + mv)
            out.cols.append(c)
            out.ttags.append(g)
            out.emissive.append(e)
        for c, rad, col, tag in self.balls:
            out.balls.append((c * s @ r.T + mv, rad * s, col, tag))
        if self.mass is not None:
            out.mass = (self.mass[0] * s @ r.T + mv, self.mass[1] * s)
        return out

    def extend(self, other):
        self.tris += other.tris
        self.cols += other.cols
        self.ttags += other.ttags
        self.emissive += other.emissive
        self.balls += other.balls
        if self.mass is None:
            self.mass = other.mass

    def points(self):
        pts = [t.reshape(-1, 3) for t in self.tris]
        for c, r, _, _ in self.balls:
            pts.append(c + np.array([[r, 0, 0], [-r, 0, 0], [0, r, 0], [0, -r, 0], [0, 0, r], [0, 0, -r]]))
        return np.concatenate(pts) if pts else np.zeros((0, 3))

    def render(self, fr: Frame, yaw, shadow=True):
        t, c, g, e = self.merged()
        if len(t):
            fr.draw(t, c, yaw=yaw, tags=g, emissive=e, ambient=self.ambient, diffuse=self.diffuse,
                    tone=c.mean(1) / 0.5 if self.tone else None)
        if self.balls:
            cen = np.array([b[0] for b in self.balls])
            rad = np.array([b[1] for b in self.balls])
            col = np.array([b[2] for b in self.balls])
            tg = np.array([b[3] for b in self.balls])
            fr.draw_balls(cen, rad, col, yaw=yaw, mass=self.mass, leafy=self.leafy, rag=self.rag, seed=self.seed, tags=tg)
        if shadow and not self.no_shadow:
            if len(t):
                fr.cast(t, yaw=yaw)
            if self.balls:
                fr.cast_balls(cen, rad * 0.95, yaw=yaw)


def fit_scale(scenes_by_yaw, w, h, ax, ay, shadow=True, margin=1.0):
    """The largest scale at which every (scene, yaw) fits the w x h frame around the ground point (ax, ay), with its shadow."""
    best = np.inf
    for sc, yaw in scenes_by_yaw:
        p = sc.points()
        if not len(p):
            continue
        m = eye_matrix(yaw)
        e = p @ m.T / UNITS_PER_PX
        xs, ys = [e[:, 0]], [e[:, 1]]
        if shadow and not sc.no_shadow:
            foot = p.copy()
            foot[:, 1] = 0
            fe = foot @ m.T / UNITS_PER_PX
            xs.append(fe[:, 0] + p[:, 1] / UNITS_PER_PX * SHADOW_DX)
            ys.append(fe[:, 1] + p[:, 1] / UNITS_PER_PX * SHADOW_DY)
        x = np.concatenate(xs)
        y = np.concatenate(ys)
        for need, avail in ((-x.min(), ax - margin), (x.max(), w - ax - margin), (y.max(), ay - margin),
                            (-y.min(), h - ay - margin)):
            if need > 1e-6:
                best = min(best, avail / need)
    return best if np.isfinite(best) else 1.0


# ---- solids ------------------------------------------------------------------------------------------------------------------


def frame_of(a, b):
    """An orthonormal frame with its third axis along b - a."""
    d = np.asarray(b, float) - np.asarray(a, float)
    ln = np.linalg.norm(d)
    z = d / ln if ln > 1e-9 else np.array([0, 1, 0.0])
    up = np.array([0, 1, 0.0]) if abs(z[1]) < 0.9 else np.array([1, 0, 0.0])
    x = np.cross(up, z)
    x /= np.linalg.norm(x)
    y = np.cross(z, x)
    return x, y, z, ln


def tube(a, b, ra, rb, sides=8, caps=True):
    """A tapered cylinder from a (radius ra) to b (radius rb): triangles."""
    x, y, z, ln = frame_of(a, b)
    a = np.asarray(a, float)
    b = np.asarray(b, float)
    ang = np.linspace(0, 2 * np.pi, sides + 1)[:-1]
    ring = np.stack([np.cos(ang), np.sin(ang)], 1)
    pa = a + ra * (ring[:, :1] * x + ring[:, 1:] * y)
    pb = b + rb * (ring[:, :1] * x + ring[:, 1:] * y)
    tris = []
    for i in range(sides):
        j = (i + 1) % sides
        tris.append([pa[i], pa[j], pb[j]])
        tris.append([pa[i], pb[j], pb[i]])
        if caps:
            tris.append([b, pb[i], pb[j]])
            tris.append([a, pa[j], pa[i]])
    return np.array(tris)


def box(c, size, rot_y=0.0):
    """An axis-aligned (then turned about y) box centred at c with size (sx, sy, sz): triangles."""
    sx, sy, sz = np.asarray(size, float) / 2
    v = np.array([[x, y, z] for x in (-sx, sx) for y in (-sy, sy) for z in (-sz, sz)])
    a = np.radians(rot_y)
    r = np.array([[np.cos(a), 0, np.sin(a)], [0, 1, 0], [-np.sin(a), 0, np.cos(a)]])
    v = v @ r.T + np.asarray(c, float)
    faces = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]
    tris = []
    for a_, b_, c_, d_ in faces:
        tris.append([v[a_], v[b_], v[c_]])
        tris.append([v[a_], v[c_], v[d_]])
    return np.array(tris)


def ellipsoid(c, r, n=10, m=7):
    """An ellipsoid centred at c with radii r = (rx, ry, rz): triangles."""
    c = np.asarray(c, float)
    r = np.asarray(r, float)
    th = np.linspace(0, np.pi, m + 1)
    ph = np.linspace(0, 2 * np.pi, n + 1)
    P = np.array([[np.sin(t) * np.cos(p), np.cos(t), np.sin(t) * np.sin(p)] for t in th for p in ph]).reshape(m + 1, n + 1, 3)
    P = P * r + c
    tris = []
    for i in range(m):
        for j in range(n):
            a, b, cc, d = P[i, j], P[i, j + 1], P[i + 1, j + 1], P[i + 1, j]
            if i > 0:
                tris.append([a, b, cc])
            if i < m - 1:
                tris.append([a, cc, d])
    return np.array(tris)


def quad(a, b, c, d):
    return np.array([[a, b, c], [a, c, d]], float)
