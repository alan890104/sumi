#!/usr/bin/env python3
"""Generate cross-validation data for Rust VBx vs Python pyannote.

Usage:
    python scripts/gen_vbx_crossval.py

Outputs vbx_crossval.json with:
  - Raw 256-dim embeddings (two speakers, 5 each)
  - PLDA-transformed 128-dim features
  - VBx gamma/pi output
  - Final cluster labels
"""

import json
import numpy as np
from scipy.special import softmax

# Load real PLDA params
import os
from huggingface_hub import hf_hub_download

token = os.environ.get("HF_TOKEN", None)
repo_id = "pyannote/speaker-diarization-community-1"
xvec_path = hf_hub_download(repo_id, "xvec_transform.npz", subfolder="plda", token=token)
plda_path = hf_hub_download(repo_id, "plda.npz", subfolder="plda", token=token)

xvec_data = np.load(xvec_path)
plda_data = np.load(plda_path)

mean1 = xvec_data["mean1"].astype(np.float64).ravel()
mean2 = xvec_data["mean2"].astype(np.float64).ravel()
lda = xvec_data["lda"].astype(np.float64)

d_in = lda.shape[0]
d_out = lda.shape[1]

mu = plda_data["mu"].astype(np.float64).ravel()
tr = plda_data["tr"].astype(np.float64)
psi = plda_data["psi"].astype(np.float64).ravel()

from scipy.linalg import eigh
W = np.linalg.inv(tr.T @ tr)
B = np.linalg.inv((tr.T / psi) @ tr)
acvar, wccn = eigh(B, W)
plda_psi = acvar[::-1]
plda_tr_mat = wccn.T[::-1]

phi = plda_psi[:d_out]

print(f"d_in={d_in}, d_out={d_out}")
print(f"phi range: [{phi.min():.6f}, {phi.max():.6f}]")


def xvec_transform(emb):
    """Apply xvec_tf (same as pyannote's vbx_setup lambda)."""
    x = emb - mean1
    x = x / np.linalg.norm(x)
    x = np.sqrt(d_in) * x
    x = x @ lda
    x = x - mean2
    x = x / np.linalg.norm(x)
    x = np.sqrt(d_out) * x
    return x


def plda_transform(x):
    """Apply plda_tf (same as pyannote's vbx_setup lambda)."""
    return (x - mu) @ plda_tr_mat[:d_out, :d_out].T


def full_transform(emb):
    return plda_transform(xvec_transform(emb))


def vbx_reference(features, phi, init_labels, k, fa=0.07, fb=0.8,
                  max_iters=20, epsilon=1e-4, init_smoothing=7.0):
    """Pure-numpy VBx implementation matching pyannote's vbx.py."""
    t, d = features.shape
    fa_fb = fa / fb

    V = np.sqrt(phi)
    rho = features * V
    G = -0.5 * (np.sum(features ** 2, axis=1) + d * np.log(2 * np.pi))

    qinit = np.zeros((t, k), dtype=np.float64)
    qinit[np.arange(t), init_labels] = 1.0
    gamma = softmax(qinit * init_smoothing, axis=1)

    pi = np.ones(k) / k
    prev_elbo = -np.inf

    for iteration in range(max_iters):
        N_s = gamma.sum(axis=0)
        invL = 1.0 / (1.0 + fa_fb * N_s[:, None] * phi[None, :])
        alpha = fa_fb * invL * (gamma.T @ rho)

        log_p = fa * (rho @ alpha.T - 0.5 * ((invL + alpha ** 2) @ phi) + G[:, None])

        lpi = np.log(pi + 1e-8)
        log_p_x = np.array([
            np.log(np.sum(np.exp(log_p[ti] + lpi - np.max(log_p[ti] + lpi)))) + np.max(log_p[ti] + lpi)
            for ti in range(t)
        ])
        gamma = np.exp(log_p + lpi[None, :] - log_p_x[:, None])
        pi = gamma.sum(axis=0)
        pi = pi / pi.sum()

        elbo = np.sum(log_p_x) + fb * 0.5 * np.sum(np.log(invL) - invL - alpha ** 2 + 1)
        if iteration > 0 and abs(elbo - prev_elbo) < epsilon:
            print(f"  Converged at iter {iteration} (ELBO={elbo:.6f})")
            break
        prev_elbo = elbo

    return gamma, pi


# Generate test embeddings: two speakers with distinct directions
np.random.seed(42)

# Speaker A: dominant in first 128 dims
raw_A = np.random.randn(5, d_in).astype(np.float64) * 0.3
raw_A[:, :128] += 1.0  # bias first half

# Speaker B: dominant in last 128 dims
raw_B = np.random.randn(5, d_in).astype(np.float64) * 0.3
raw_B[:, 128:] += 1.0  # bias second half

raw_embs = np.vstack([raw_A, raw_B])
print(f"\nRaw embeddings: shape={raw_embs.shape}")

# Transform all embeddings
transformed = np.array([full_transform(e) for e in raw_embs])
print(f"Transformed: shape={transformed.shape}")
print(f"  norms: {np.linalg.norm(transformed, axis=1)}")

# AHC initialization (simulate: first 5 = cluster 0, last 5 = cluster 1)
# In practice we'd use cosine AHC, but for cross-validation we use known labels
init_labels = np.array([0] * 5 + [1] * 5)
k = 2

print("\n=== VBx with pyannote defaults (Fa=0.07, Fb=0.8) ===")
gamma, pi_arr = vbx_reference(transformed, phi, init_labels, k, fa=0.07, fb=0.8)

print(f"  pi = {pi_arr}")
for i in range(10):
    assigned = np.argmax(gamma[i])
    print(f"  emb {i}: cluster {assigned} (γ={gamma[i, assigned]:.6f})")

# Also test xvec_transform alone (for Rust cross-validation)
xvec_results = np.array([xvec_transform(e) for e in raw_embs])

# Export
data = {
    "d_in": int(d_in),
    "d_out": int(d_out),
    "raw_embeddings": raw_embs.tolist(),
    "xvec_transformed": xvec_results.tolist(),
    "plda_transformed": transformed.tolist(),
    "init_labels": init_labels.tolist(),
    "k": k,
    "fa": 0.07,
    "fb": 0.8,
    "gamma": gamma.tolist(),
    "pi": pi_arr.tolist(),
    "phi_first8": phi[:8].tolist(),
}

with open("vbx_crossval.json", "w") as f:
    json.dump(data, f, indent=2)

print(f"\nWrote vbx_crossval.json")
print(f"  phi[:8] = {phi[:8]}")
