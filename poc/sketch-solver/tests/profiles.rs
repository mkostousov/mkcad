//! K-3: profile detection (F.8) — SK-01/10/11/12 and the F73–F76 reference cases.

use sketch_solver::profiles::{TOL_LIN, detect_profiles};
use sketch_solver::{Code, Curve, CurveGeom, CurveId, Point, PointId, Provenance, Sketch};
use std::f64::consts::PI;

#[derive(Default)]
struct B {
    s: Sketch,
    n: u64,
}

impl B {
    fn pt(&mut self, x: f64, y: f64) -> PointId {
        self.n += 1;
        let id = PointId(self.n);
        self.s.points.insert(id, Point { id, x, y, prov: Provenance::default() });
        id
    }

    fn curve(&mut self, geom: CurveGeom, construction: bool) -> CurveId {
        self.n += 1;
        let id = CurveId(self.n);
        self.s.curves.insert(id, Curve { id, geom, construction, prov: Provenance::default() });
        id
    }

    fn line(&mut self, a: (f64, f64), b: (f64, f64)) -> CurveId {
        let (a, b) = (self.pt(a.0, a.1), self.pt(b.0, b.1));
        self.curve(CurveGeom::Line { a, b }, false)
    }

    fn construction_line(&mut self, a: (f64, f64), b: (f64, f64)) -> CurveId {
        let (a, b) = (self.pt(a.0, a.1), self.pt(b.0, b.1));
        self.curve(CurveGeom::Line { a, b }, true)
    }

    fn circle(&mut self, c: (f64, f64), r: f64) -> CurveId {
        let center = self.pt(c.0, c.1);
        self.curve(CurveGeom::Circle { center, r }, false)
    }

    /// Counter-clockwise arc from angle `a0` to `a1` (radians).
    fn arc(&mut self, c: (f64, f64), r: f64, a0: f64, a1: f64) -> CurveId {
        let center = self.pt(c.0, c.1);
        let start = self.pt(c.0 + r * a0.cos(), c.1 + r * a0.sin());
        let end = self.pt(c.0 + r * a1.cos(), c.1 + r * a1.sin());
        self.curve(CurveGeom::Arc { center, start, end, r }, false)
    }

    /// Rectangle with shared corner points.
    fn rect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        let p = [self.pt(x0, y0), self.pt(x1, y0), self.pt(x1, y1), self.pt(x0, y1)];
        for i in 0..4 {
            self.curve(CurveGeom::Line { a: p[i], b: p[(i + 1) % 4] }, false);
        }
    }

    fn polyline_closed(&mut self, pts: &[(f64, f64)]) {
        for i in 0..pts.len() {
            self.line(pts[i], pts[(i + 1) % pts.len()]);
        }
    }
}

fn areas(b: &B) -> Vec<f64> {
    let mut a: Vec<f64> = detect_profiles(&b.s).profiles.iter().map(|p| p.area).collect();
    a.sort_by(|x, y| x.total_cmp(y));
    a
}

fn close(a: f64, b: f64, rel: f64) -> bool {
    (a - b).abs() <= rel * b.abs().max(1e-300)
}

#[test]
fn sk01_rectangle_is_one_profile_800() {
    let mut b = B::default();
    b.rect(0.0, 0.0, 40.0, 20.0);
    let set = detect_profiles(&b.s);
    assert_eq!(set.profiles.len(), 1);
    assert_eq!(set.profiles[0].loop_count(), 1);
    assert!(close(set.profiles[0].area, 800.0, 1e-12));
    assert_eq!(set.profiles[0].outer.pieces.len(), 4);
    assert!(set.diagnostics.is_empty());
}

#[test]
fn sk10_chain_gap_closes_at_1e5_not_at_2e5() {
    for (gap, expected) in [(1e-5, 1), (2e-5, 0), (0.0, 1), (5e-6, 1), (1.5e-5 + 1e-6, 0)] {
        let mut b = B::default();
        b.line((0.0, 0.0), (10.0, 0.0));
        b.line((10.0, 0.0), (10.0, 10.0));
        b.line((10.0, 10.0), (0.0, 10.0));
        b.line((0.0, 10.0), (0.0, gap)); // end gap `gap` to the first point
        let n = detect_profiles(&b.s).profiles.len();
        assert_eq!(n, expected, "gap {gap}");
    }
    // The sketch's own points are untouched (F76: no auto-merge).
    assert_eq!(TOL_LIN, 1e-5);
}

#[test]
fn sk11_scale_and_offset_ranges_give_exact_areas() {
    let s = 2f64.powi(-10); // exactly representable "1e-3 mm"
    for (side, off) in [(s, 0.0), (1e6, 0.0), (s, 1e6), (1e6, 1e6)] {
        let mut b = B::default();
        b.rect(off, off, off + side, off + side);
        let a = areas(&b);
        assert_eq!(a.len(), 1, "side {side} offset {off}");
        // Side as represented in the stored coordinates.
        let w = (off + side) - off;
        assert!(close(a[0], w * w, 1e-12), "side {side} offset {off}: {} vs {}", a[0], w * w);
    }
    // Decimal 1e-3 at offset 1e6: the area follows the coordinates as stored.
    let mut b = B::default();
    b.rect(1e6, 1e6, 1e6 + 1e-3, 1e6 + 1e-3);
    let w = (1e6 + 1e-3) - 1e6;
    assert!(close(areas(&b)[0], w * w, 1e-12));
}

#[test]
fn sk12_rect_contains_circle_contains_circle() {
    let mut b = B::default();
    b.rect(-50.0, -50.0, 50.0, 50.0);
    b.circle((0.0, 0.0), 30.0);
    b.circle((0.0, 0.0), 10.0);
    let set = detect_profiles(&b.s);
    assert_eq!(set.profiles.len(), 3);
    let mut loops: Vec<usize> = set.profiles.iter().map(|p| p.loop_count()).collect();
    loops.sort();
    assert_eq!(loops, [1, 2, 2]);
    let a = areas(&b);
    assert!(close(a[0], PI * 100.0, 1e-12));
    assert!(close(a[1], PI * (900.0 - 100.0), 1e-12));
    assert!(close(a[2], 10000.0 - PI * 900.0, 1e-12));
}

#[test]
fn f74_overlapping_circles_give_lens_and_two_crescents() {
    let mut b = B::default();
    b.circle((0.0, 0.0), 10.0);
    b.circle((10.0, 0.0), 10.0);
    let a = areas(&b);
    assert_eq!(a.len(), 3);
    let lens = 2.0 * 100.0 * (0.5f64).acos() - 5.0 * (300.0f64).sqrt();
    assert!(close(a[0], lens, 1e-9), "{} vs {lens}", a[0]);
    assert!(close(a[1], PI * 100.0 - lens, 1e-9));
    assert!(close(a[2], PI * 100.0 - lens, 1e-9));
}

#[test]
fn f74_tangent_circles_give_two_regions() {
    let mut external = B::default();
    external.circle((0.0, 0.0), 10.0);
    external.circle((20.0, 0.0), 10.0);
    assert_eq!(areas(&external).len(), 2);

    // Internally tangent: inner disc + the crescent around it.
    let mut internal = B::default();
    internal.circle((0.0, 0.0), 10.0);
    internal.circle((5.0, 0.0), 5.0);
    let a = areas(&internal);
    assert_eq!(a.len(), 2);
    assert!(close(a[0], PI * 25.0, 1e-9));
    assert!(close(a[1], PI * (100.0 - 25.0), 1e-9));
}

#[test]
fn f74_bowtie_gives_two_triangles() {
    let mut b = B::default();
    b.polyline_closed(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]);
    let a = areas(&b);
    assert_eq!(a.len(), 2);
    assert!(close(a[0], 25.0, 1e-12) && close(a[1], 25.0, 1e-12));
}

#[test]
fn f74_rectangle_crossed_by_unconnected_line_gives_two_halves() {
    let mut b = B::default();
    b.rect(0.0, 0.0, 10.0, 10.0);
    b.line((5.0, -5.0), (5.0, 15.0));
    let a = areas(&b);
    assert_eq!(a.len(), 2);
    assert!(close(a[0], 50.0, 1e-12) && close(a[1], 50.0, 1e-12));
}

#[test]
fn f74_overshooting_tic_tac_toe_gives_one_inner_square() {
    let mut b = B::default();
    b.line((3.0, -5.0), (3.0, 15.0));
    b.line((7.0, -5.0), (7.0, 15.0));
    b.line((-5.0, 3.0), (15.0, 3.0));
    b.line((-5.0, 7.0), (15.0, 7.0));
    let a = areas(&b);
    assert_eq!(a.len(), 1);
    assert!(close(a[0], 16.0, 1e-12));
}

#[test]
fn f75_exact_duplicates_yield_one_profile_and_a_warning() {
    let mut c = B::default();
    c.circle((0.0, 0.0), 10.0);
    c.circle((0.0, 0.0), 10.0);
    let set = detect_profiles(&c.s);
    assert_eq!(set.profiles.len(), 1);
    assert!(set.diagnostics.iter().any(|d| d.code == Code::GeometryDuplicate));

    let mut r = B::default();
    r.rect(0.0, 0.0, 10.0, 5.0);
    r.rect(0.0, 0.0, 10.0, 5.0);
    let set = detect_profiles(&r.s);
    assert_eq!(set.profiles.len(), 1);
    assert!(close(set.profiles[0].area, 50.0, 1e-12));
    assert_eq!(set.diagnostics.iter().filter(|d| d.code == Code::GeometryDuplicate).count(), 4);
}

#[test]
fn construction_curves_and_open_chains_give_no_region() {
    let mut b = B::default();
    b.rect(0.0, 0.0, 10.0, 10.0);
    b.construction_line((5.0, -1.0), (5.0, 11.0)); // would split the rectangle if it counted
    assert_eq!(areas(&b), [100.0]);

    let mut open = B::default();
    open.line((0.0, 0.0), (10.0, 0.0));
    open.line((10.0, 0.0), (10.0, 10.0));
    open.line((10.0, 10.0), (0.0, 10.0));
    assert!(detect_profiles(&open.s).profiles.is_empty());
}

#[test]
fn arcs_bound_regions_with_exact_areas() {
    // Half disc: upper semicircle + its diameter.
    let mut b = B::default();
    b.arc((0.0, 0.0), 10.0, 0.0, PI);
    b.line((-10.0, 0.0), (10.0, 0.0));
    let a = areas(&b);
    assert_eq!(a.len(), 1);
    assert!(close(a[0], 50.0 * PI, 1e-12), "{}", a[0]);

    // Three quarters of a disc (arc sweep > pi) closed by two radii.
    let mut q = B::default();
    q.arc((0.0, 0.0), 10.0, 0.0, 1.5 * PI);
    q.line((0.0, -10.0), (0.0, 0.0));
    q.line((0.0, 0.0), (10.0, 0.0));
    let a = areas(&q);
    assert_eq!(a.len(), 1);
    assert!(close(a[0], 0.75 * PI * 100.0, 1e-12), "{}", a[0]);

    // Slot: two lines + two half circles.
    let mut s = B::default();
    s.line((0.0, 5.0), (20.0, 5.0));
    s.line((20.0, -5.0), (0.0, -5.0));
    s.arc((20.0, 0.0), 5.0, -PI / 2.0, PI / 2.0);
    s.arc((0.0, 0.0), 5.0, PI / 2.0, 1.5 * PI);
    let a = areas(&s);
    assert_eq!(a.len(), 1);
    assert!(close(a[0], 200.0 + PI * 25.0, 1e-12), "{}", a[0]);
}

#[test]
fn bridge_between_nested_loops_does_not_change_regions() {
    let mut b = B::default();
    b.rect(-50.0, -50.0, 50.0, 50.0);
    b.circle((0.0, 0.0), 10.0);
    b.line((10.0, 0.0), (50.0, 0.0)); // joins the circle to the rectangle
    let set = detect_profiles(&b.s);
    assert_eq!(set.profiles.len(), 2);
    let a = areas(&b);
    assert!(close(a[0], PI * 100.0, 1e-12));
    assert!(close(a[1], 10000.0 - PI * 100.0, 1e-12));
}

#[test]
fn degenerate_curves_are_reported_and_skipped() {
    let mut b = B::default();
    b.rect(0.0, 0.0, 10.0, 10.0);
    b.line((3.0, 3.0), (3.0, 3.0));
    b.circle((5.0, 5.0), 0.0);
    let set = detect_profiles(&b.s);
    assert_eq!(set.profiles.len(), 1);
    assert_eq!(set.diagnostics.iter().filter(|d| d.code == Code::GeometryDegenerate).count(), 2);
}

#[test]
fn detection_is_deterministic_and_independent_of_curve_order() {
    let mut b = B::default();
    b.rect(0.0, 0.0, 30.0, 20.0);
    b.circle((10.0, 10.0), 4.0);
    b.line((20.0, -5.0), (20.0, 25.0));
    let first = detect_profiles(&b.s);
    assert_eq!(first, detect_profiles(&b.s));

    // Same geometry, curves inserted in the opposite order (different ids).
    let mut r = B::default();
    r.line((20.0, -5.0), (20.0, 25.0));
    r.circle((10.0, 10.0), 4.0);
    r.rect(0.0, 0.0, 30.0, 20.0);
    let mut a = areas(&b);
    let mut c = areas(&r);
    a.sort_by(f64::total_cmp);
    c.sort_by(f64::total_cmp);
    assert_eq!(a.len(), c.len());
    for (x, y) in a.iter().zip(&c) {
        assert!(close(*x, *y, 1e-12));
    }
}

#[test]
fn thousand_curve_sketch_is_fast() {
    // 250 separate squares (1000 lines) plus 40 crossing guide lines through the grid.
    let mut b = B::default();
    for i in 0..25 {
        for j in 0..10 {
            let (x, y) = (i as f64 * 12.0, j as f64 * 12.0);
            b.rect(x, y, x + 10.0, y + 10.0);
        }
    }
    for k in 0..20 {
        b.line((k as f64 * 15.0 + 1.0, -5.0), (k as f64 * 15.0 + 1.0, 125.0));
    }
    let curves = b.s.curves.len();
    let t = std::time::Instant::now();
    let set = detect_profiles(&b.s);
    let ms = t.elapsed().as_secs_f64() * 1e3;
    println!("PERF curves={curves} profiles={} ms={ms:.1}", set.profiles.len());
    assert!(set.profiles.len() >= 250);
    if !cfg!(debug_assertions) {
        assert!(ms < 100.0, "{ms} ms");
    }
}

/// Deterministic xorshift in [0, 1).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[test]
fn random_chords_tile_the_rectangle_exactly() {
    for seed in 1..=20u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let (w, h) = (40.0, 25.0);
        let mut b = B::default();
        b.rect(0.0, 0.0, w, h);
        let on_boundary = |t: f64| {
            let per = 2.0 * (w + h);
            let s = t * per;
            if s < w {
                (s, 0.0)
            } else if s < w + h {
                (w, s - w)
            } else if s < 2.0 * w + h {
                (w - (s - w - h), h)
            } else {
                (0.0, h - (s - 2.0 * w - h))
            }
        };
        for _ in 0..12 {
            b.line(on_boundary(rng.next()), on_boundary(rng.next()));
        }
        let set = detect_profiles(&b.s);
        let total: f64 = set.profiles.iter().map(|p| p.area).sum();
        assert!(close(total, w * h, 1e-9), "seed {seed}: total {total}");
        assert!(set.profiles.iter().all(|p| p.area > 0.0 && p.holes.is_empty()), "seed {seed}");
    }
}

#[test]
fn random_chords_tile_the_disc_exactly() {
    for seed in 1..=20u64 {
        let mut rng = Rng(seed.wrapping_mul(0xD1B5_4A32_D192_ED03));
        let r = 17.0;
        let mut b = B::default();
        b.circle((3.0, -2.0), r);
        for _ in 0..8 {
            let (a0, a1) = (rng.next() * 2.0 * PI, rng.next() * 2.0 * PI);
            b.line((3.0 + r * a0.cos(), -2.0 + r * a0.sin()), (3.0 + r * a1.cos(), -2.0 + r * a1.sin()));
        }
        let set = detect_profiles(&b.s);
        let total: f64 = set.profiles.iter().map(|p| p.area).sum();
        assert!(close(total, PI * r * r, 1e-9), "seed {seed}: total {total}");
    }
}
