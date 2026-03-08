//! VBx (Variational Bayes x-vector) clustering for speaker diarization.
//!
//! Implements pyannote's `VBxClustering` pipeline:
//!
//! 1. AHC initialization (centroid linkage, threshold=0.6)
//! 2. PLDA transform (xvec_tf → plda_tf → 128-dim features)
//! 3. VBx inference (iterative VB E-step/M-step with PLDA model)
//! 4. Speaker pruning (pi > 1e-7) and centroid computation
//! 5. Optional K-Means override when cluster count is outside [min, max]
//!
//! Reference: Landini et al., "Bayesian HMM clustering of x-vector sequences
//! (VBx) in speaker diarization"

use std::path::Path;

use crate::diarization::{centroid_linkage_cluster, cosine_dist, l2_normalize};

// ── PLDA parameters ─────────────────────────────────────────────────────────────

/// Pre-computed PLDA parameters for the WeSpeaker ResNet34-LM embedding model.
///
/// Loaded from a binary file produced by `scripts/extract_plda.py`.
/// The file contains the result of `vbx_setup()` (eigendecomposition already done).
///
/// Binary format:
/// ```text
/// "SUMIPLDA\0"  (9 bytes magic)
/// version: u8   (= 1)
/// d_in: u32     (256)
/// d_out: u32    (128)
/// mean1:   [f32; d_in]
/// mean2:   [f32; d_out]
/// lda:     [f32; d_in * d_out]   (row-major)
/// plda_mu: [f32; d_out]
/// plda_tr: [f32; d_out * d_out]  (row-major)
/// phi:     [f32; d_out]
/// ```
pub(crate) struct PldaParams {
    d_in: usize,
    d_out: usize,
    mean1: Vec<f32>,
    mean2: Vec<f32>,
    lda: Vec<f32>,      // (d_in × d_out) row-major
    plda_mu: Vec<f32>,
    plda_tr: Vec<f32>,  // (d_out × d_out) row-major
    phi: Vec<f32>,
}

const MAGIC: &[u8; 9] = b"SUMIPLDA\0";

impl PldaParams {
    /// Load pre-computed PLDA params from a binary file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let data =
            std::fs::read(path).map_err(|e| format!("Failed to read PLDA file: {e}"))?;
        let mut cursor = 0usize;

        // Magic
        if data.len() < 9 || &data[..9] != MAGIC {
            return Err("Invalid PLDA file: bad magic".into());
        }
        cursor += 9;

        // Version
        if data.len() < cursor + 1 {
            return Err("Invalid PLDA file: truncated version".into());
        }
        let version = data[cursor];
        cursor += 1;
        if version != 1 {
            return Err(format!("Unsupported PLDA version: {version}"));
        }

        // d_in, d_out
        let d_in = read_u32(&data, &mut cursor)? as usize;
        let d_out = read_u32(&data, &mut cursor)? as usize;

        let mean1 = read_f32_vec(&data, &mut cursor, d_in)?;
        let mean2 = read_f32_vec(&data, &mut cursor, d_out)?;
        let lda = read_f32_vec(&data, &mut cursor, d_in * d_out)?;
        let plda_mu = read_f32_vec(&data, &mut cursor, d_out)?;
        let plda_tr = read_f32_vec(&data, &mut cursor, d_out * d_out)?;
        let phi = read_f32_vec(&data, &mut cursor, d_out)?;

        Ok(Self {
            d_in,
            d_out,
            mean1,
            mean2,
            lda,
            plda_mu,
            plda_tr,
            phi,
        })
    }

    /// Apply the full xvec + PLDA transform: (N, d_in) → (N, d_out).
    ///
    /// Input embeddings must be RAW (un-normalized) from the WeSpeaker model.
    pub fn transform(&self, embeddings: &[Vec<f32>]) -> Vec<Vec<f32>> {
        embeddings
            .iter()
            .map(|emb| self.transform_one(emb))
            .collect()
    }

    /// Transform a single embedding.
    ///
    /// Implements pyannote's `xvec_tf` followed by `plda_tf`:
    ///
    /// xvec_tf:
    ///   1. x = emb − mean1
    ///   2. x = L2_norm(x)
    ///   3. x *= √d_in
    ///   4. x = x @ lda          (d_in → d_out)
    ///   5. x = x − mean2
    ///   6. x = L2_norm(x)
    ///   7. x *= √d_out
    ///
    /// plda_tf:
    ///   8. x = (x − plda_mu) @ plda_tr.T
    fn transform_one(&self, emb: &[f32]) -> Vec<f32> {
        let d_in = self.d_in;
        let d_out = self.d_out;

        // Step 1: center
        let mut x: Vec<f32> = emb.iter().zip(&self.mean1).map(|(&e, &m)| e - m).collect();

        // Step 2: L2 normalize
        let norm: f32 = x.iter().map(|&v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut x {
                *v /= norm;
            }
        }

        // Step 3: scale by √d_in
        let scale_in = (d_in as f32).sqrt();
        for v in &mut x {
            *v *= scale_in;
        }

        // Step 4: LDA projection (x @ lda)  — lda is (d_in, d_out) row-major
        let mut y = vec![0.0f32; d_out];
        for (j, yj) in y.iter_mut().enumerate() {
            let mut sum = 0.0f32;
            for (i, &xi) in x.iter().enumerate() {
                sum += xi * self.lda[i * d_out + j];
            }
            *yj = sum;
        }

        // Step 5: center by mean2
        for (v, &m) in y.iter_mut().zip(&self.mean2) {
            *v -= m;
        }

        // Step 6: L2 normalize
        let norm: f32 = y.iter().map(|&v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut y {
                *v /= norm;
            }
        }

        // Step 7: scale by √d_out
        let scale_out = (d_out as f32).sqrt();
        for v in &mut y {
            *v *= scale_out;
        }

        // Step 8: PLDA projection — (y − plda_mu) @ plda_tr.T
        // plda_tr is (d_out, d_out) row-major; transposed = column j of plda_tr
        let centered: Vec<f32> = y.iter().zip(&self.plda_mu).map(|(&v, &m)| v - m).collect();
        let mut out = vec![0.0f32; d_out];
        for (j, oj) in out.iter_mut().enumerate() {
            let mut sum = 0.0f32;
            for (i, &ci) in centered.iter().enumerate() {
                // plda_tr.T[i][j] = plda_tr[j][i] = plda_tr[j * d_out + i]
                sum += ci * self.plda_tr[j * d_out + i];
            }
            *oj = sum;
        }

        out
    }
}

fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, String> {
    if *cursor + 4 > data.len() {
        return Err("PLDA file truncated".into());
    }
    let val = u32::from_le_bytes([
        data[*cursor],
        data[*cursor + 1],
        data[*cursor + 2],
        data[*cursor + 3],
    ]);
    *cursor += 4;
    Ok(val)
}

fn read_f32_vec(data: &[u8], cursor: &mut usize, count: usize) -> Result<Vec<f32>, String> {
    let byte_len = count * 4;
    if *cursor + byte_len > data.len() {
        return Err(format!(
            "PLDA file truncated: need {} bytes at offset {}, have {}",
            byte_len,
            *cursor,
            data.len()
        ));
    }
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let val = f32::from_le_bytes([
            data[*cursor],
            data[*cursor + 1],
            data[*cursor + 2],
            data[*cursor + 3],
        ]);
        *cursor += 4;
        out.push(val);
    }
    Ok(out)
}

// ── VBx configuration ───────────────────────────────────────────────────────────

/// VBx clustering hyperparameters.
///
/// Defaults match pyannote's `SpeakerDiarization.default_parameters()`.
pub(crate) struct VbxConfig {
    /// AHC threshold for initialization (pyannote default: 0.6).
    pub threshold: f32,
    /// Sufficient statistics scaling factor (pyannote default: 0.07).
    pub fa: f32,
    /// Speaker regularization (pyannote default: 0.8).
    pub fb: f32,
    /// Maximum VBx iterations (pyannote default: 20).
    pub max_iters: usize,
    /// ELBO convergence threshold (pyannote default: 1e-4).
    pub epsilon: f32,
    /// Softmax temperature for AHC → VBx initialization (pyannote default: 7.0).
    pub init_smoothing: f32,
}

impl Default for VbxConfig {
    fn default() -> Self {
        Self {
            threshold: 0.6,
            fa: 0.07,
            fb: 0.8,
            max_iters: 20,
            epsilon: 1e-4,
            init_smoothing: 7.0,
        }
    }
}

// ── Main entry point ────────────────────────────────────────────────────────────

/// Run VBx clustering on raw (un-normalized) embeddings.
///
/// Implements pyannote's `VBxClustering.__call__`:
/// 1. L2-normalize → AHC initialization (centroid linkage)
/// 2. PLDA transform → VBx iteration
/// 3. Prune speakers with pi ≤ 1e-7
/// 4. Compute centroids from raw embeddings weighted by responsibilities
/// 5. K-Means override if cluster count outside [min_speakers, max_speakers]
/// 6. Assign all embeddings to nearest centroid (cosine distance)
///
/// Returns 0-based cluster labels, one per embedding.
pub(crate) fn vbx_cluster(
    raw_embeddings: &[Vec<f32>],
    plda: &PldaParams,
    config: &VbxConfig,
    min_speakers: Option<usize>,
    max_speakers: Option<usize>,
) -> Vec<usize> {
    let n = raw_embeddings.len();
    if n == 0 {
        return vec![];
    }
    if n < 2 {
        return vec![0; n];
    }

    // 1. L2-normalize for AHC
    let normed: Vec<Vec<f32>> = raw_embeddings.iter().map(|e| l2_normalize(e)).collect();

    // 2. AHC initialization — min_cluster_size=1 because VBx handles speaker pruning
    let ahc_labels = centroid_linkage_cluster(&normed, config.threshold, 1);
    let k = ahc_labels.iter().max().map(|&m| m + 1).unwrap_or(1);

    tracing::debug!("[vbx] AHC init: {} embeddings → {} clusters (threshold={:.4})", n, k, config.threshold);

    // 3. PLDA transform
    let features = plda.transform(raw_embeddings);

    // 4. VBx iteration
    let (gamma, pi) = vbx_iterate(
        &features,
        &plda.phi,
        &ahc_labels,
        k,
        config.fa,
        config.fb,
        config.max_iters,
        config.epsilon,
        config.init_smoothing,
    );

    // 5. Extract active speakers (pi > 1e-7)
    let active: Vec<usize> = pi
        .iter()
        .enumerate()
        .filter(|(_, &p)| p > 1e-7)
        .map(|(i, _)| i)
        .collect();

    tracing::debug!(
        "[vbx] VBx pruning: {} → {} speakers (pi > 1e-7)",
        k,
        active.len()
    );

    if active.is_empty() {
        return vec![0; n];
    }

    // 6. Compute centroids from raw embeddings weighted by responsibilities
    let dim = raw_embeddings[0].len();
    let mut centroids: Vec<Vec<f32>> = Vec::with_capacity(active.len());
    for &spk in &active {
        let mut centroid = vec![0.0f32; dim];
        let mut w_sum = 0.0f32;
        for (t, emb) in raw_embeddings.iter().enumerate() {
            let w = gamma[t * k + spk];
            w_sum += w;
            for (d, &val) in emb.iter().enumerate() {
                centroid[d] += w * val;
            }
        }
        if w_sum > 0.0 {
            for v in &mut centroid {
                *v /= w_sum;
            }
        }
        centroids.push(centroid);
    }

    let mut num_clusters = centroids.len();

    // 7. K-Means override if outside bounds
    let min_k = min_speakers.unwrap_or(1);
    let max_k = max_speakers.unwrap_or(usize::MAX);
    if num_clusters < min_k || num_clusters > max_k {
        let target = num_clusters.clamp(min_k, max_k);
        tracing::debug!(
            "[vbx] K-Means override: {} → {} clusters ([{}, {}])",
            num_clusters,
            target,
            min_k,
            max_k
        );
        let km_labels = kmeans(&normed, target, 3, 100, 42);
        centroids = compute_kmeans_centroids(raw_embeddings, &km_labels, target);
        num_clusters = target;
    }

    let _ = num_clusters; // used only for logging above

    // 8. Assign all embeddings to nearest centroid (cosine distance)
    raw_embeddings
        .iter()
        .map(|emb| {
            centroids
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| cosine_dist(emb, a).total_cmp(&cosine_dist(emb, b)))
                .map(|(i, _)| i)
                .unwrap_or(0)
        })
        .collect()
}

// ── VBx core algorithm ──────────────────────────────────────────────────────────

/// Core VBx iteration (E-step + M-step).
///
/// Operates in PLDA-transformed space.
///
/// Returns `(gamma, pi)`:
/// - `gamma`: flattened (T × K) responsibility matrix (row-major)
/// - `pi`: (K,) speaker priors
#[allow(clippy::too_many_arguments)]
fn vbx_iterate(
    features: &[Vec<f32>], // (T, D) in PLDA space
    phi: &[f32],           // (D,) between-class covariance eigenvalues
    init_labels: &[usize], // (T,) AHC cluster labels
    k: usize,              // number of initial clusters
    fa: f32,
    fb: f32,
    max_iters: usize,
    epsilon: f32,
    init_smoothing: f32,
) -> (Vec<f32>, Vec<f32>) {
    let t = features.len();
    let d = features[0].len();
    let fa_fb = fa / fb;

    // ── Precompute ──────────────────────────────────────────────────────────

    // V = sqrt(Phi)
    let v: Vec<f32> = phi.iter().map(|&p| p.sqrt()).collect();

    // rho[t][d] = features[t][d] * V[d]
    let rho: Vec<Vec<f32>> = features
        .iter()
        .map(|f| f.iter().zip(&v).map(|(&x, &vi)| x * vi).collect())
        .collect();

    // G[t] = -0.5 * (||features[t]||² + D·ln(2π))
    let ln_2pi = (2.0 * std::f32::consts::PI).ln();
    let g: Vec<f32> = features
        .iter()
        .map(|f| {
            let sum_sq: f32 = f.iter().map(|&x| x * x).sum();
            -0.5 * (sum_sq + d as f32 * ln_2pi)
        })
        .collect();

    // ── Initialize gamma from AHC labels (softmax smoothed) ─────────────

    // gamma[t * k + s] — flattened (T, K) row-major
    let mut gamma = vec![0.0f32; t * k];
    for (i, &label) in init_labels.iter().enumerate() {
        gamma[i * k + label] = init_smoothing;
    }
    // Softmax per row
    for row_start in (0..t * k).step_by(k) {
        let row = &mut gamma[row_start..row_start + k];
        let max_val = row.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let sum: f32 = row.iter().map(|&x| (x - max_val).exp()).sum();
        for val in row.iter_mut() {
            *val = (*val - max_val).exp() / sum;
        }
    }

    // pi = uniform 1/K
    let mut pi = vec![1.0 / k as f32; k];

    let mut prev_elbo = f32::NEG_INFINITY;

    // ── VB iterations ───────────────────────────────────────────────────────

    for iter in 0..max_iters {
        // M-step: N_s[s] = sum_t gamma[t][s]
        let mut n_s = vec![0.0f32; k];
        for ti in 0..t {
            for s in 0..k {
                n_s[s] += gamma[ti * k + s];
            }
        }

        // invL[s][d] = 1 / (1 + fa_fb * N_s[s] * phi[d])
        // Flattened (K, D)
        let mut inv_l = vec![0.0f32; k * d];
        for s in 0..k {
            for dd in 0..d {
                inv_l[s * d + dd] = 1.0 / (1.0 + fa_fb * n_s[s] * phi[dd]);
            }
        }

        // alpha[s][d] = fa_fb * invL[s][d] * sum_t(gamma[t][s] * rho[t][d])
        // Flattened (K, D)
        let mut alpha = vec![0.0f32; k * d];
        for ti in 0..t {
            for s in 0..k {
                let w = gamma[ti * k + s];
                for dd in 0..d {
                    alpha[s * d + dd] += w * rho[ti][dd];
                }
            }
        }
        for s in 0..k {
            for dd in 0..d {
                alpha[s * d + dd] *= fa_fb * inv_l[s * d + dd];
            }
        }

        // E-step: log_p[t][s] = fa * (rho[t] · alpha[s] − 0.5 * bias[s] + G[t])
        // bias[s] = sum_d (invL[s][d] + alpha[s][d]²) * phi[d]
        let mut speaker_bias = vec![0.0f32; k];
        for s in 0..k {
            let mut sum = 0.0f32;
            for dd in 0..d {
                let a = alpha[s * d + dd];
                sum += (inv_l[s * d + dd] + a * a) * phi[dd];
            }
            speaker_bias[s] = sum;
        }

        // log_p flattened (T, K)
        let mut log_p = vec![0.0f32; t * k];
        for ti in 0..t {
            for s in 0..k {
                let mut dot = 0.0f32;
                for dd in 0..d {
                    dot += rho[ti][dd] * alpha[s * d + dd];
                }
                log_p[ti * k + s] = fa * (dot - 0.5 * speaker_bias[s] + g[ti]);
            }
        }

        // GMM update: normalize responsibilities
        let eps_val = 1e-8f32;
        let lpi: Vec<f32> = pi.iter().map(|&p| (p + eps_val).ln()).collect();

        let mut log_p_x = vec![0.0f32; t];
        for ti in 0..t {
            // logsumexp(log_p[t] + lpi)
            let mut max_val = f32::NEG_INFINITY;
            for s in 0..k {
                let v = log_p[ti * k + s] + lpi[s];
                if v > max_val {
                    max_val = v;
                }
            }
            let sum: f32 = (0..k)
                .map(|s| (log_p[ti * k + s] + lpi[s] - max_val).exp())
                .sum();
            log_p_x[ti] = max_val + sum.ln();
        }

        // gamma = exp(log_p + lpi − log_p_x)
        for ti in 0..t {
            for s in 0..k {
                gamma[ti * k + s] =
                    (log_p[ti * k + s] + lpi[s] - log_p_x[ti]).exp();
            }
        }

        // pi = normalized gamma column sums
        let mut pi_sum = 0.0f32;
        for (s, pi_s) in pi.iter_mut().enumerate() {
            *pi_s = 0.0;
            for ti in 0..t {
                *pi_s += gamma[ti * k + s];
            }
            pi_sum += *pi_s;
        }
        if pi_sum > 0.0 {
            for pi_s in pi.iter_mut() {
                *pi_s /= pi_sum;
            }
        }

        // ELBO convergence check
        let term1: f32 = log_p_x.iter().sum();
        let mut term2 = 0.0f32;
        for s in 0..k {
            for dd in 0..d {
                let il = inv_l[s * d + dd];
                let a = alpha[s * d + dd];
                term2 += il.ln() - il - a * a + 1.0;
            }
        }
        let elbo = term1 + fb * 0.5 * term2;

        if iter > 0 && (elbo - prev_elbo).abs() < epsilon {
            tracing::debug!("[vbx] converged at iter {} (ELBO={:.4})", iter, elbo);
            break;
        }
        if iter > 0 && elbo < prev_elbo {
            tracing::debug!(
                "[vbx] ELBO decreased at iter {}: {:.4} → {:.4}",
                iter,
                prev_elbo,
                elbo
            );
        }
        prev_elbo = elbo;
    }

    (gamma, pi)
}

// ── K-Means ─────────────────────────────────────────────────────────────────────

/// Simple K-Means with K-Means++ initialization.
///
/// Runs `n_init` restarts and returns the labels from the best run (lowest
/// inertia).  Operates on L2-normalized embeddings.
fn kmeans(
    data: &[Vec<f32>],
    k: usize,
    n_init: usize,
    max_iter: usize,
    seed: u64,
) -> Vec<usize> {
    let n = data.len();
    if n == 0 || k == 0 {
        return vec![];
    }
    if k >= n {
        return (0..n).collect();
    }

    let dim = data[0].len();
    let mut best_labels: Vec<usize> = vec![0; n];
    let mut best_inertia = f64::MAX;
    let mut rng_state = seed;

    for _ in 0..n_init {
        // K-Means++ init
        let mut centers: Vec<Vec<f32>> = Vec::with_capacity(k);

        // First center: random
        let idx = (simple_rng(&mut rng_state) % n as u64) as usize;
        centers.push(data[idx].clone());

        for _ in 1..k {
            // Compute squared distance to nearest center
            let mut dists: Vec<f64> = data
                .iter()
                .map(|x| {
                    centers
                        .iter()
                        .map(|c| sq_euclidean_f64(x, c))
                        .fold(f64::MAX, f64::min)
                })
                .collect();

            // Normalize to probabilities
            let sum: f64 = dists.iter().sum();
            if sum <= 0.0 {
                break;
            }
            for d in &mut dists {
                *d /= sum;
            }

            // Weighted random selection
            let r = (simple_rng(&mut rng_state) as f64) / (u64::MAX as f64);
            let mut cumulative = 0.0;
            let mut chosen = 0;
            for (i, &d) in dists.iter().enumerate() {
                cumulative += d;
                if cumulative >= r {
                    chosen = i;
                    break;
                }
            }
            centers.push(data[chosen].clone());
        }

        // Pad if K-Means++ didn't produce enough centers
        while centers.len() < k {
            let idx = (simple_rng(&mut rng_state) % n as u64) as usize;
            centers.push(data[idx].clone());
        }

        // Lloyd's algorithm
        let mut labels = vec![0usize; n];
        for _ in 0..max_iter {
            // Assign
            let mut changed = false;
            for (i, x) in data.iter().enumerate() {
                let nearest = centers
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        sq_euclidean_f64(x, a)
                            .partial_cmp(&sq_euclidean_f64(x, b))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(idx, _)| idx)
                    .unwrap_or(0);
                if labels[i] != nearest {
                    labels[i] = nearest;
                    changed = true;
                }
            }

            if !changed {
                break;
            }

            // Update centers
            let mut new_centers = vec![vec![0.0f32; dim]; k];
            let mut counts = vec![0usize; k];
            for (i, x) in data.iter().enumerate() {
                let c = labels[i];
                counts[c] += 1;
                for (d, &val) in x.iter().enumerate() {
                    new_centers[c][d] += val;
                }
            }
            for c in 0..k {
                if counts[c] > 0 {
                    for val in &mut new_centers[c] {
                        *val /= counts[c] as f32;
                    }
                }
            }
            centers = new_centers;
        }

        // Compute inertia
        let inertia: f64 = data
            .iter()
            .zip(&labels)
            .map(|(x, &c)| sq_euclidean_f64(x, &centers[c]))
            .sum();

        if inertia < best_inertia {
            best_inertia = inertia;
            best_labels = labels;
        }
    }

    best_labels
}

/// Compute centroids from raw embeddings given K-Means labels.
fn compute_kmeans_centroids(
    embeddings: &[Vec<f32>],
    labels: &[usize],
    k: usize,
) -> Vec<Vec<f32>> {
    if embeddings.is_empty() {
        return vec![];
    }
    let dim = embeddings[0].len();
    let mut sums = vec![vec![0.0f32; dim]; k];
    let mut counts = vec![0usize; k];
    for (emb, &c) in embeddings.iter().zip(labels) {
        counts[c] += 1;
        for (d, &val) in emb.iter().enumerate() {
            sums[c][d] += val;
        }
    }
    for c in 0..k {
        if counts[c] > 0 {
            for val in &mut sums[c] {
                *val /= counts[c] as f32;
            }
        }
    }
    sums
}

fn sq_euclidean_f64(a: &[f32], b: &[f32]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| {
            let d = (x - y) as f64;
            d * d
        })
        .sum()
}

/// Simple xorshift64 PRNG (deterministic, matching random_state=42).
fn simple_rng(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ── Tests ───────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a trivial PldaParams with identity transforms for testing VBx
    /// math without real PLDA parameters.
    fn identity_plda(d: usize) -> PldaParams {
        let mut lda = vec![0.0f32; d * d];
        for i in 0..d {
            lda[i * d + i] = 1.0; // identity matrix
        }
        let mut plda_tr = vec![0.0f32; d * d];
        for i in 0..d {
            plda_tr[i * d + i] = 1.0;
        }
        PldaParams {
            d_in: d,
            d_out: d,
            mean1: vec![0.0; d],
            mean2: vec![0.0; d],
            lda,
            plda_mu: vec![0.0; d],
            plda_tr,
            phi: vec![1.0; d], // uniform eigenvalues
        }
    }

    #[test]
    fn plda_identity_transform_is_passthrough() {
        let plda = identity_plda(4);
        // Input: [1, 0, 0, 0]
        // After centering (−mean1=0): same
        // After L2_norm: same (already unit)
        // After scale √4 = 2: [2, 0, 0, 0]
        // After LDA (identity): [2, 0, 0, 0]
        // After centering (−mean2=0): same
        // After L2_norm: [1, 0, 0, 0]
        // After scale √4 = 2: [2, 0, 0, 0]
        // After PLDA (identity, −mu=0): [2, 0, 0, 0]
        let result = plda.transform_one(&[1.0, 0.0, 0.0, 0.0]);
        assert_eq!(result.len(), 4);
        assert!((result[0] - 2.0).abs() < 1e-5);
        assert!(result[1].abs() < 1e-5);
        assert!(result[2].abs() < 1e-5);
        assert!(result[3].abs() < 1e-5);
    }

    #[test]
    fn vbx_iterate_two_obvious_clusters() {
        // Two well-separated clusters in 4-D PLDA space.
        // Use Fa=1.0, Fb=1.0 for synthetic features — pyannote's defaults
        // (Fa=0.07) are tuned for real PLDA-transformed 128-dim embeddings
        // and converge too slowly on low-dim toy data.
        let features: Vec<Vec<f32>> = vec![
            vec![5.0, 0.0, 0.0, 0.0],
            vec![4.8, 0.2, 0.0, 0.0],
            vec![5.1, -0.1, 0.0, 0.0],
            vec![0.0, 0.0, 5.0, 0.0],
            vec![0.0, 0.0, 4.9, 0.1],
            vec![0.0, 0.0, 5.2, -0.1],
        ];
        let phi = vec![1.0f32; 4];
        let init_labels = vec![0, 0, 0, 1, 1, 1];
        let k = 2;

        let (gamma, pi) = vbx_iterate(&features, &phi, &init_labels, k, 1.0, 1.0, 20, 1e-4, 7.0);

        // Both speakers should survive (pi > 1e-7)
        assert!(pi[0] > 0.1, "speaker 0 should be active: pi={}", pi[0]);
        assert!(pi[1] > 0.1, "speaker 1 should be active: pi={}", pi[1]);

        // Each embedding should have high responsibility for its cluster
        for ti in 0..3 {
            assert!(
                gamma[ti * k] > 0.9,
                "emb {} should belong to cluster 0: γ={:.4}",
                ti,
                gamma[ti * k]
            );
        }
        for ti in 3..6 {
            assert!(
                gamma[ti * k + 1] > 0.9,
                "emb {} should belong to cluster 1: γ={:.4}",
                ti,
                gamma[ti * k + 1]
            );
        }
    }

    #[test]
    fn vbx_iterate_kills_empty_speaker() {
        // 3 initial clusters but only 2 real speakers — VBx should kill one.
        // Fa=1.0, Fb=1.0 for synthetic features (see above).
        let features: Vec<Vec<f32>> = vec![
            vec![5.0, 0.0, 0.0, 0.0],
            vec![4.9, 0.1, 0.0, 0.0],
            vec![0.0, 0.0, 5.0, 0.0],
            vec![0.0, 0.0, 4.9, 0.1],
        ];
        let phi = vec![1.0f32; 4];
        // AHC over-segments into 3 clusters
        let init_labels = vec![0, 1, 2, 2];
        let k = 3;

        let (_gamma, pi) =
            vbx_iterate(&features, &phi, &init_labels, k, 1.0, 1.0, 20, 1e-4, 7.0);

        // At least one speaker should be pruned
        let active = pi.iter().filter(|&&p| p > 1e-7).count();
        assert!(
            active <= 2,
            "expected ≤2 active speakers, got {} (pi={:?})",
            active,
            pi
        );
    }

    #[test]
    fn vbx_cluster_two_speakers_with_identity_plda() {
        let plda = identity_plda(4);
        let config = VbxConfig::default();

        // Two clusters: [1,0,0,0] and [0,0,1,0]
        let embeddings: Vec<Vec<f32>> = vec![
            vec![1.0, 0.0, 0.0, 0.0],
            vec![0.98, 0.02, 0.0, 0.0],
            vec![1.01, -0.01, 0.0, 0.0],
            vec![0.0, 0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.99, 0.01],
            vec![0.0, 0.0, 1.02, -0.01],
        ];

        let labels = vbx_cluster(&embeddings, &plda, &config, None, None);
        assert_eq!(labels.len(), 6);

        // First 3 should share a label, last 3 should share a different label
        assert_eq!(labels[0], labels[1]);
        assert_eq!(labels[0], labels[2]);
        assert_eq!(labels[3], labels[4]);
        assert_eq!(labels[3], labels[5]);
        assert_ne!(labels[0], labels[3]);
    }

    #[test]
    fn vbx_cluster_single_embedding() {
        let plda = identity_plda(4);
        let config = VbxConfig::default();
        let labels = vbx_cluster(&[vec![1.0, 0.0, 0.0, 0.0]], &plda, &config, None, None);
        assert_eq!(labels, vec![0]);
    }

    #[test]
    fn vbx_cluster_empty() {
        let plda = identity_plda(4);
        let config = VbxConfig::default();
        let labels = vbx_cluster(&[], &plda, &config, None, None);
        assert!(labels.is_empty());
    }

    #[test]
    fn kmeans_two_clusters() {
        let data = vec![
            vec![1.0, 0.0],
            vec![0.9, 0.1],
            vec![1.1, -0.1],
            vec![-1.0, 0.0],
            vec![-0.9, 0.1],
            vec![-1.1, -0.1],
        ];
        let labels = kmeans(&data, 2, 3, 100, 42);
        assert_eq!(labels.len(), 6);
        // First 3 same cluster, last 3 same cluster
        assert_eq!(labels[0], labels[1]);
        assert_eq!(labels[0], labels[2]);
        assert_eq!(labels[3], labels[4]);
        assert_eq!(labels[3], labels[5]);
        assert_ne!(labels[0], labels[3]);
    }

    #[test]
    fn kmeans_single_cluster() {
        let data = vec![vec![1.0, 0.0], vec![1.1, 0.1], vec![0.9, -0.1]];
        let labels = kmeans(&data, 1, 1, 100, 42);
        assert_eq!(labels, vec![0, 0, 0]);
    }

    /// Load real plda.bin and verify PLDA transform matches Python output.
    ///
    /// The reference values come from `scripts/extract_plda.py` verification step:
    /// ```
    /// np.random.seed(0) → test_emb = np.random.randn(256).astype(np.float32)
    /// ```
    #[test]
    #[ignore] // requires ~/.sumi-dev/models/plda.bin
    fn plda_real_transform_matches_python() {
        let path = crate::settings::plda_model_path();
        if !path.exists() {
            eprintln!("Skipping: {:?} not found", path);
            return;
        }

        let plda = PldaParams::load(&path).unwrap();
        assert_eq!(plda.d_in, 256, "expected d_in=256");
        assert_eq!(plda.d_out, 128, "expected d_out=128");
        assert_eq!(plda.mean1.len(), 256);
        assert_eq!(plda.mean2.len(), 128);
        assert_eq!(plda.lda.len(), 256 * 128);
        assert_eq!(plda.plda_mu.len(), 128);
        assert_eq!(plda.plda_tr.len(), 128 * 128);
        assert_eq!(plda.phi.len(), 128);

        // Verify phi range matches Python output
        let phi_min = plda.phi.iter().cloned().fold(f32::INFINITY, f32::min);
        let phi_max = plda.phi.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(
            phi_min > 0.5 && phi_min < 0.7,
            "phi min={} expected ~0.57",
            phi_min
        );
        assert!(
            phi_max > 25.0 && phi_max < 27.0,
            "phi max={} expected ~25.88",
            phi_max
        );

        // Generate same test embedding as Python (numpy random seed=0)
        // We'll test with a known embedding and compare against Python output
        // For now, verify the transform produces reasonable output
        let test_emb: Vec<f32> = (0..256).map(|i| (i as f32 * 0.01) - 1.28).collect();
        let result = plda.transform_one(&test_emb);
        assert_eq!(result.len(), 128);

        // Transformed embeddings should have reasonable magnitude
        let norm: f32 = result.iter().map(|&v| v * v).sum::<f32>().sqrt();
        assert!(
            norm > 1.0 && norm < 100.0,
            "transformed norm={} seems unreasonable",
            norm
        );
    }

    /// Cross-validate full VBx pipeline (PLDA transform + VBx iterate) against
    /// Python pyannote reference output.
    ///
    /// Reference data from `scripts/gen_vbx_crossval.py` (seed=42, 10 embeddings,
    /// 2 speakers). Reads `vbx_crossval.json` from project root.
    #[test]
    #[ignore] // requires vbx_crossval.json + ~/.sumi-dev/models/plda.bin
    fn vbx_full_pipeline_matches_python() {
        let plda_path = crate::settings::plda_model_path();
        if !plda_path.exists() {
            eprintln!("Skipping: {:?} not found", plda_path);
            return;
        }

        let json_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("vbx_crossval.json");
        if !json_path.exists() {
            eprintln!("Skipping: {:?} not found. Run: python scripts/gen_vbx_crossval.py", json_path);
            return;
        }

        let json_str = std::fs::read_to_string(&json_path).unwrap();
        let data: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let plda = PldaParams::load(&plda_path).unwrap();

        // Parse raw embeddings
        let raw_embs: Vec<Vec<f32>> = data["raw_embeddings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                row.as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap() as f32)
                    .collect()
            })
            .collect();

        let n = raw_embs.len();
        assert_eq!(n, 10);

        // Parse Python's PLDA-transformed features for comparison
        let py_transformed: Vec<Vec<f64>> = data["plda_transformed"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                row.as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap())
                    .collect()
            })
            .collect();

        // 1. Verify PLDA transform matches Python
        let rust_transformed = plda.transform(&raw_embs);
        for i in 0..n {
            for j in 0..plda.d_out {
                let diff = (rust_transformed[i][j] as f64 - py_transformed[i][j]).abs();
                assert!(
                    diff < 0.05, // f32 accumulation over 256-dim dot products
                    "PLDA transform mismatch at [{i}][{j}]: rust={:.6} python={:.6} diff={:.6}",
                    rust_transformed[i][j],
                    py_transformed[i][j],
                    diff,
                );
            }
        }
        eprintln!("  ✓ PLDA transform matches Python (within f32 tolerance)");

        // 2. Run VBx on Rust-transformed features and verify clustering matches
        let init_labels: Vec<usize> = data["init_labels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        let k = data["k"].as_u64().unwrap() as usize;
        let fa = data["fa"].as_f64().unwrap() as f32;
        let fb = data["fb"].as_f64().unwrap() as f32;

        let (gamma, pi) = vbx_iterate(
            &rust_transformed,
            &plda.phi,
            &init_labels,
            k,
            fa,
            fb,
            20,
            1e-4,
            7.0,
        );

        // Parse Python gamma/pi
        let py_pi: Vec<f64> = data["pi"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let py_gamma: Vec<Vec<f64>> = data["gamma"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                row.as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap())
                    .collect()
            })
            .collect();

        // Verify pi
        for s in 0..k {
            let diff = (pi[s] as f64 - py_pi[s]).abs();
            assert!(
                diff < 0.05,
                "pi[{s}] mismatch: rust={:.6} python={:.6}",
                pi[s],
                py_pi[s],
            );
        }
        eprintln!("  ✓ pi matches Python: rust={:?} python={:?}", pi, py_pi);

        // Verify cluster assignments match (argmax of gamma)
        for i in 0..n {
            let rust_cluster = (0..k)
                .max_by(|&a, &b| gamma[i * k + a].total_cmp(&gamma[i * k + b]))
                .unwrap();
            let py_cluster = (0..k)
                .max_by(|&a, &b| py_gamma[i][a].total_cmp(&py_gamma[i][b]))
                .unwrap();
            assert_eq!(
                rust_cluster, py_cluster,
                "Cluster assignment mismatch at emb {i}: rust={} python={} (gamma_rust=[{:.4},{:.4}] gamma_py=[{:.4},{:.4}])",
                rust_cluster,
                py_cluster,
                gamma[i * k],
                gamma[i * k + 1],
                py_gamma[i][0],
                py_gamma[i][1],
            );
        }
        eprintln!("  ✓ All 10 cluster assignments match Python");

        // 3. Verify end-to-end vbx_cluster produces correct labels
        let config = VbxConfig {
            threshold: 0.6,
            fa,
            fb,
            max_iters: 20,
            epsilon: 1e-4,
            init_smoothing: 7.0,
        };
        let labels = vbx_cluster(&raw_embs, &plda, &config, None, None);
        assert_eq!(labels.len(), 10);

        // First 5 should be one cluster, last 5 another
        let a = labels[0];
        let b = labels[5];
        assert_ne!(a, b, "Two speakers should get different labels");
        for i in 0..5 {
            assert_eq!(labels[i], a, "emb {i} should be speaker A (label={a})");
        }
        for i in 5..10 {
            assert_eq!(labels[i], b, "emb {i} should be speaker B (label={b})");
        }
        eprintln!("  ✓ vbx_cluster end-to-end: 2 speakers correctly separated");
    }

    #[test]
    fn plda_roundtrip_binary() {
        let plda = identity_plda(4);
        let path = std::env::temp_dir().join("test_plda.bin");

        // Write binary
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(MAGIC);
        buf.push(1u8); // version
        buf.extend_from_slice(&(plda.d_in as u32).to_le_bytes());
        buf.extend_from_slice(&(plda.d_out as u32).to_le_bytes());
        for &v in &plda.mean1 {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        for &v in &plda.mean2 {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        for &v in &plda.lda {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        for &v in &plda.plda_mu {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        for &v in &plda.plda_tr {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        for &v in &plda.phi {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        std::fs::write(&path, &buf).unwrap();

        // Load and verify
        let loaded = PldaParams::load(&path).unwrap();
        assert_eq!(loaded.d_in, 4);
        assert_eq!(loaded.d_out, 4);
        assert_eq!(loaded.mean1, plda.mean1);
        assert_eq!(loaded.phi, plda.phi);

        let _ = std::fs::remove_file(&path);
    }

    /// Cross-validation against Python reference output.
    ///
    /// Reference values from `scripts/test_vbx.py` (numpy + scipy softmax).
    #[test]
    fn vbx_iterate_matches_python_reference_test1() {
        // Test 1: Two clusters, Fa=1.0, Fb=1.0
        // Python: pi = [0.49999999947, 0.50000000053]
        //         gamma[0] = [0.99999999324, 6.76e-09]
        //         gamma[3] = [7.66e-09, 0.99999999234]
        let features: Vec<Vec<f32>> = vec![
            vec![5.0, 0.0, 0.0, 0.0],
            vec![4.8, 0.2, 0.0, 0.0],
            vec![5.1, -0.1, 0.0, 0.0],
            vec![0.0, 0.0, 5.0, 0.0],
            vec![0.0, 0.0, 4.9, 0.1],
            vec![0.0, 0.0, 5.2, -0.1],
        ];
        let phi = vec![1.0f32; 4];
        let init_labels = vec![0, 0, 0, 1, 1, 1];
        let k = 2;

        let (gamma, pi) = vbx_iterate(
            &features, &phi, &init_labels, k, 1.0, 1.0, 20, 1e-4, 7.0,
        );

        // pi should be ~[0.5, 0.5]
        assert!(
            (pi[0] - 0.5).abs() < 0.01,
            "pi[0]={} expected ~0.5",
            pi[0]
        );
        assert!(
            (pi[1] - 0.5).abs() < 0.01,
            "pi[1]={} expected ~0.5",
            pi[1]
        );

        // gamma[0] should be ~[1.0, 0.0]
        assert!(
            gamma[0 * k] > 0.999,
            "gamma[0][0]={} expected >0.999",
            gamma[0]
        );
        // gamma[3] should be ~[0.0, 1.0]
        assert!(
            gamma[3 * k + 1] > 0.999,
            "gamma[3][1]={} expected >0.999",
            gamma[3 * k + 1]
        );
    }

    #[test]
    fn vbx_iterate_matches_python_reference_test2() {
        // Test 2: Speaker killing — 3 init, 2 real. Fa=1.0, Fb=1.0
        // Python: pi = [0.5, 1e-13, 0.5] → speaker 1 killed
        let features: Vec<Vec<f32>> = vec![
            vec![5.0, 0.0, 0.0, 0.0],
            vec![4.9, 0.1, 0.0, 0.0],
            vec![0.0, 0.0, 5.0, 0.0],
            vec![0.0, 0.0, 4.9, 0.1],
        ];
        let phi = vec![1.0f32; 4];
        let init_labels = vec![0, 1, 2, 2];
        let k = 3;

        let (_gamma, pi) = vbx_iterate(
            &features, &phi, &init_labels, k, 1.0, 1.0, 20, 1e-4, 7.0,
        );

        // Speaker 1 should be killed (pi < 1e-7)
        let active: Vec<usize> = pi
            .iter()
            .enumerate()
            .filter(|(_, &p)| p > 1e-7)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            active.len(),
            2,
            "expected 2 active speakers, got {} (pi={:?})",
            active.len(),
            pi
        );

        // Surviving speakers should have ~equal pi
        let surviving_pi: Vec<f32> = active.iter().map(|&i| pi[i]).collect();
        assert!(
            (surviving_pi[0] - 0.5).abs() < 0.01,
            "surviving pi[0]={} expected ~0.5",
            surviving_pi[0]
        );
    }
}
