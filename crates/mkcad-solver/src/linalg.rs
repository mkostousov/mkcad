//! Sparse incremental LDLᵀ of `A = J·W⁻¹·Jᵀ (+ λI)` with in-order pivot dropping.
//!
//! Rows are added in a fixed order (internal → constraints by ID → soft goals). A row whose Schur-complement
//! pivot vanishes is linearly dependent on the rows kept before it; it is dropped and its *fundamental
//! circuit* (the unique minimal dependent set inside kept rows ∪ {row}) is recorded. One factorization
//! therefore yields: rank (→ DOF), the redundant/conflicting rows with their minimal sets, and the
//! least-norm Gauss–Newton step `Δx = −W⁻¹Jᵀ(JW⁻¹Jᵀ)⁻¹r` (LM when λ > 0, by the push-through identity).
//!
//! `L` is stored column-wise as sparse lists; work is proportional to the fill, which stays small because
//! constraints touch few variables and IDs follow creation order (locality).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Sparse Jacobian row: (local column, value).
pub type SRow = Vec<(usize, f64)>;

#[derive(Clone, Debug, Default)]
pub struct Factor {
    /// Per input row: index among kept rows.
    pub pos: Vec<Option<usize>>,
    /// Kept index → input row.
    pub kept_rows: Vec<usize>,
    /// Column j of the unit lower-triangular L: (kept index i > j, L_ij).
    cols: Vec<Vec<(usize, f64)>>,
    d: Vec<f64>,
    /// Dropped input row → input rows of its fundamental circuit (excluding itself), ascending.
    pub circuits: Vec<(usize, Vec<usize>)>,
    /// Per local column: (kept index, J value) of the kept rows touching it.
    col_rows: Vec<Vec<(usize, f64)>>,
    winv: Vec<f64>,
}

/// Options of [`factor`].
#[derive(Clone, Copy, Debug)]
pub struct FactorOpts {
    pub lambda: f64,
    /// Relative pivot tolerance: row dropped when `d_k ≤ tol·A_kk`.
    pub rel_tol: f64,
    pub circuits: bool,
}

pub fn factor(rows: &[SRow], winv: &[f64], opts: FactorOpts) -> Factor {
    let ncols = winv.len();
    let mut f = Factor {
        pos: vec![None; rows.len()],
        col_rows: vec![Vec::new(); ncols],
        winv: winv.to_vec(),
        ..Default::default()
    };
    let mut y: Vec<f64> = Vec::new();
    let mut mark: Vec<bool> = Vec::new();
    let mut touched: Vec<usize> = Vec::new();
    for (k, row) in rows.iter().enumerate() {
        let r = f.kept_rows.len();
        y.resize(r, 0.0);
        mark.resize(r, false);
        let akk: f64 = row.iter().map(|&(c, v)| v * v * winv[c]).sum();
        // a = A[K, k] scattered into y, then forward solve L·y = a in ascending order.
        let mut heap = BinaryHeap::new();
        for &(c, v) in row {
            for &(i, u) in &f.col_rows[c] {
                if !mark[i] {
                    mark[i] = true;
                    touched.push(i);
                    heap.push(Reverse(i));
                }
                y[i] += v * u * winv[c];
            }
        }
        while let Some(Reverse(j)) = heap.pop() {
            let yj = y[j];
            if yj == 0.0 {
                continue;
            }
            for &(i, lij) in &f.cols[j] {
                if !mark[i] {
                    mark[i] = true;
                    touched.push(i);
                    heap.push(Reverse(i));
                }
                y[i] -= lij * yj;
            }
        }
        touched.sort_unstable();
        let mut dk = akk + opts.lambda;
        for &j in &touched {
            dk -= y[j] * y[j] / f.d[j];
        }
        let dependent = opts.lambda == 0.0 && (akk == 0.0 || dk <= opts.rel_tol * akk);
        if dependent {
            if opts.circuits {
                let members = f.circuit(&y, &touched, r);
                f.circuits.push((k, members));
            }
        } else {
            for &j in &touched {
                if y[j] != 0.0 {
                    f.cols[j].push((r, y[j] / f.d[j]));
                }
            }
            f.cols.push(Vec::new());
            f.d.push(dk);
            f.pos[k] = Some(r);
            f.kept_rows.push(k);
            for &(c, v) in row {
                f.col_rows[c].push((r, v));
            }
        }
        for &j in &touched {
            y[j] = 0.0;
            mark[j] = false;
        }
        touched.clear();
    }
    f
}

impl Factor {
    pub fn rank(&self) -> usize {
        self.kept_rows.len()
    }

    /// Coefficients c with `row ≈ Σ c_j·row_j` over kept rows < r: `c = L⁻ᵀ D⁻¹ y`.
    fn circuit(&self, y: &[f64], touched: &[usize], r: usize) -> Vec<usize> {
        if touched.is_empty() {
            return Vec::new();
        }
        let lo = touched[0];
        let mut c = vec![0.0; r - lo];
        for j in (lo..r).rev() {
            let mut cj = y[j] / self.d[j];
            for &(i, lij) in &self.cols[j] {
                if i < r {
                    cj -= lij * c[i - lo];
                }
            }
            c[j - lo] = cj;
        }
        let max = c.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let mut out: Vec<usize> =
            (lo..r).filter(|&j| c[j - lo].abs() > 1e-9 * max).map(|j| self.kept_rows[j]).collect();
        out.sort_unstable();
        out
    }

    /// Solves `A_KK·z = b_K` (b indexed by input row; only kept rows used). Returns z per kept index.
    pub fn solve(&self, b: &[f64]) -> Vec<f64> {
        let r = self.rank();
        let mut z: Vec<f64> = self.kept_rows.iter().map(|&k| b[k]).collect();
        for j in 0..r {
            let zj = z[j];
            if zj != 0.0 {
                for &(i, lij) in &self.cols[j] {
                    z[i] -= lij * zj;
                }
            }
        }
        for j in 0..r {
            z[j] /= self.d[j];
        }
        for j in (0..r).rev() {
            let mut zj = z[j];
            for &(i, lij) in &self.cols[j] {
                zj -= lij * z[i];
            }
            z[j] = zj;
        }
        z
    }

    /// Least-norm step `Δx = −W⁻¹·J_Kᵀ·A_KK⁻¹·r_K`.
    pub fn step(&self, rows: &[SRow], resid: &[f64]) -> Vec<f64> {
        let z = self.solve(resid);
        let mut dx = vec![0.0; self.winv.len()];
        for (ki, &k) in self.kept_rows.iter().enumerate() {
            for &(c, v) in &rows[k] {
                dx[c] -= self.winv[c] * v * z[ki];
            }
        }
        dx
    }

    /// Number of extra independent directions that selecting `cols` adds to the kept row space:
    /// `rank([J; E]) − rank(J)` = DOF left in those variables. Requires a factor with unit weights.
    pub fn free_dims(&self, cols: &[usize]) -> usize {
        let t = cols.len();
        if t == 0 {
            return 0;
        }
        let r = self.rank();
        // y_p = L⁻¹·a_p with a_p = A[K, e_p] = J[K, col_p].
        let mut ys: Vec<Vec<f64>> = Vec::with_capacity(t);
        for &c in cols {
            let mut y = vec![0.0; r];
            for &(i, v) in &self.col_rows[c] {
                y[i] += v * self.winv[c];
            }
            for j in 0..r {
                let yj = y[j];
                if yj != 0.0 {
                    for &(i, lij) in &self.cols[j] {
                        y[i] -= lij * yj;
                    }
                }
            }
            ys.push(y);
        }
        // Schur complement S = E·W⁻¹·Eᵀ − Yᵀ·D⁻¹·Y, rank by pivoted Cholesky.
        let mut s = vec![vec![0.0; t]; t];
        for p in 0..t {
            for q in 0..t {
                let mut v = if cols[p] == cols[q] { self.winv[cols[p]] } else { 0.0 };
                for j in 0..r {
                    v -= ys[p][j] * ys[q][j] / self.d[j];
                }
                s[p][q] = v;
            }
        }
        let mut rank = 0;
        let mut active: Vec<usize> = (0..t).collect();
        while !active.is_empty() {
            let (ai, &piv) =
                active.iter().enumerate().max_by(|a, b| s[*a.1][*a.1].total_cmp(&s[*b.1][*b.1])).expect("non-empty");
            let pv = s[piv][piv];
            if pv <= 1e-9 * self.winv[cols[piv]] {
                break;
            }
            rank += 1;
            active.remove(ai);
            for &i in &active {
                for &j in &active {
                    s[i][j] -= s[i][piv] * s[piv][j] / pv;
                }
            }
        }
        rank
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPTS: FactorOpts = FactorOpts { lambda: 0.0, rel_tol: 1e-10, circuits: true };

    #[test]
    fn detects_dependent_rows_and_circuits() {
        // r0 = x0 − x1, r1 = x1 − x2, r2 = x0 − x2 (= r0 + r1), r3 = x3
        let rows =
            vec![vec![(0, 1.0), (1, -1.0)], vec![(1, 1.0), (2, -1.0)], vec![(0, 1.0), (2, -1.0)], vec![(3, 2.0)]];
        let f = factor(&rows, &[1.0; 4], OPTS);
        assert_eq!(f.rank(), 3);
        assert_eq!(f.pos[2], None);
        assert_eq!(f.circuits, vec![(2, vec![0, 1])]);
        assert_eq!(f.free_dims(&[0, 1, 2]), 1);
        assert_eq!(f.free_dims(&[3]), 0);
    }

    #[test]
    fn least_norm_step_solves_consistent_system() {
        let rows = vec![vec![(0, 1.0), (1, 1.0)], vec![(1, 1.0), (2, -2.0)]];
        let resid = [3.0, -1.0];
        let f = factor(&rows, &[1.0, 1.0, 1.0], OPTS);
        let dx = f.step(&rows, &resid);
        for (row, r) in rows.iter().zip(resid) {
            let jdx: f64 = row.iter().map(|&(c, v)| v * dx[c]).sum();
            assert!((jdx + r).abs() < 1e-12);
        }
        // Least norm: dx lies in the row space → orthogonal to the null vector (2, −2, −1).
        assert!((2.0 * dx[0] - 2.0 * dx[1] - dx[2]).abs() < 1e-12);
    }

    #[test]
    fn rank_matches_svd_on_random_sparse_matrices() {
        let mut seed = 7u64;
        let mut rnd = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as usize
        };
        for _ in 0..50 {
            let (m, n) = (2 + rnd() % 12, 2 + rnd() % 12);
            let mut rows: Vec<SRow> = Vec::new();
            for _ in 0..m {
                if !rows.is_empty() && rnd() % 4 == 0 {
                    // Linear combination of two earlier rows → dependent.
                    let (a, b) = (rnd() % rows.len(), rnd() % rows.len());
                    let mut dense = vec![0.0; n];
                    for &(c, v) in rows[a].iter() {
                        dense[c] += v;
                    }
                    for &(c, v) in rows[b].iter() {
                        dense[c] -= 0.5 * v;
                    }
                    rows.push(dense.iter().enumerate().filter(|e| *e.1 != 0.0).map(|(c, v)| (c, *v)).collect());
                } else {
                    let mut row: SRow = (0..3).map(|_| (rnd() % n, (rnd() % 1000) as f64 / 100.0 - 5.0)).collect();
                    row.sort_by_key(|e| e.0);
                    row.dedup_by_key(|e| e.0);
                    rows.push(row);
                }
            }
            let dense = nalgebra::DMatrix::<f64>::from_fn(m, n, |i, j| {
                rows[i].iter().filter(|e| e.0 == j).map(|e| e.1).sum::<f64>()
            });
            let f = factor(&rows, &vec![1.0; n], OPTS);
            assert_eq!(f.rank(), dense.rank(1e-9), "{rows:?}");
        }
    }
}
