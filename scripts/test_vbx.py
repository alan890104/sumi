#!/usr/bin/env python3
"""Generate VBx reference data for Rust cross-validation tests.

Usage:
    pip install pyannote.audio torch scipy numpy
    python scripts/test_vbx.py

Generates test vectors and expected outputs that can be used to verify
the Rust VBx implementation matches pyannote's exactly.
"""

import json
import numpy as np
from scipy.special import softmax


def vbx_reference(features, phi, init_labels, k, fa=0.07, fb=0.8,
                  max_iters=20, epsilon=1e-4, init_smoothing=7.0):
    """Pure-numpy VBx implementation matching pyannote's vbx.py."""
    t, d = features.shape
    fa_fb = fa / fb

    # Precompute
    V = np.sqrt(phi)
    rho = features * V  # (T, D)
    G = -0.5 * (np.sum(features ** 2, axis=1) + d * np.log(2 * np.pi))  # (T,)

    # Initialize gamma from AHC labels
    qinit = np.zeros((t, k), dtype=np.float64)
    qinit[np.arange(t), init_labels] = 1.0
    gamma = softmax(qinit * init_smoothing, axis=1)

    pi = np.ones(k) / k

    prev_elbo = -np.inf

    for iteration in range(max_iters):
        # M-step
        N_s = gamma.sum(axis=0)  # (K,)
        invL = 1.0 / (1.0 + fa_fb * N_s[:, None] * phi[None, :])  # (K, D)
        alpha = fa_fb * invL * (gamma.T @ rho)  # (K, D)

        # E-step
        log_p = fa * (rho @ alpha.T - 0.5 * ((invL + alpha ** 2) @ phi) + G[:, None])  # (T, K)

        # GMM update
        lpi = np.log(pi + 1e-8)
        log_p_x = np.array([
            np.log(np.sum(np.exp(log_p[ti] + lpi - np.max(log_p[ti] + lpi)))) + np.max(log_p[ti] + lpi)
            for ti in range(t)
        ])
        gamma = np.exp(log_p + lpi[None, :] - log_p_x[:, None])
        pi = gamma.sum(axis=0)
        pi = pi / pi.sum()

        # ELBO
        elbo = np.sum(log_p_x) + fb * 0.5 * np.sum(np.log(invL) - invL - alpha ** 2 + 1)
        if iteration > 0 and abs(elbo - prev_elbo) < epsilon:
            print(f"  Converged at iter {iteration} (ELBO={elbo:.6f})")
            break
        prev_elbo = elbo

    return gamma, pi


def main():
    np.random.seed(42)

    # Test 1: Two obvious clusters (matching Rust test with Fa=1.0, Fb=1.0)
    print("=== Test 1: Two obvious clusters (Fa=1.0, Fb=1.0) ===")
    features = np.array([
        [5.0, 0.0, 0.0, 0.0],
        [4.8, 0.2, 0.0, 0.0],
        [5.1, -0.1, 0.0, 0.0],
        [0.0, 0.0, 5.0, 0.0],
        [0.0, 0.0, 4.9, 0.1],
        [0.0, 0.0, 5.2, -0.1],
    ], dtype=np.float64)
    phi = np.ones(4, dtype=np.float64)
    init_labels = np.array([0, 0, 0, 1, 1, 1])

    gamma, pi = vbx_reference(features, phi, init_labels, 2, fa=1.0, fb=1.0)
    print(f"  pi = {pi}")
    print(f"  gamma[0] = {gamma[0]} (expect cluster 0)")
    print(f"  gamma[3] = {gamma[3]} (expect cluster 1)")

    # Test 2: Speaker killing (3 init clusters, 2 real)
    print("\n=== Test 2: Speaker killing (Fa=1.0, Fb=1.0) ===")
    features2 = np.array([
        [5.0, 0.0, 0.0, 0.0],
        [4.9, 0.1, 0.0, 0.0],
        [0.0, 0.0, 5.0, 0.0],
        [0.0, 0.0, 4.9, 0.1],
    ], dtype=np.float64)
    init_labels2 = np.array([0, 1, 2, 2])
    gamma2, pi2 = vbx_reference(features2, phi, init_labels2, 3, fa=1.0, fb=1.0)
    active = np.sum(pi2 > 1e-7)
    print(f"  pi = {pi2}")
    print(f"  active speakers = {active}")

    # Test 3: With pyannote's default hyperparameters (Fa=0.07, Fb=0.8)
    # Use 128-dim features typical of PLDA output
    print("\n=== Test 3: 128-dim PLDA-like features (Fa=0.07, Fb=0.8) ===")
    d = 128
    # Two speakers: embeddings clustered around two different directions
    spk0 = np.random.randn(5, d).astype(np.float64)
    spk0 = spk0 / np.linalg.norm(spk0, axis=1, keepdims=True) * np.sqrt(d)
    spk0 += np.array([1.0] + [0.0] * (d - 1))

    spk1 = np.random.randn(5, d).astype(np.float64)
    spk1 = spk1 / np.linalg.norm(spk1, axis=1, keepdims=True) * np.sqrt(d)
    spk1 += np.array([0.0, 1.0] + [0.0] * (d - 2))

    features3 = np.vstack([spk0, spk1])
    phi3 = np.ones(d, dtype=np.float64) * 0.5  # typical eigenvalue scale
    init_labels3 = np.array([0] * 5 + [1] * 5)

    gamma3, pi3 = vbx_reference(features3, phi3, init_labels3, 2, fa=0.07, fb=0.8)
    print(f"  pi = {pi3}")
    for i in range(10):
        assigned = np.argmax(gamma3[i])
        print(f"  emb {i}: cluster {assigned} (γ={gamma3[i, assigned]:.4f})")

    # Dump reference data as JSON for Rust tests
    ref_data = {
        "test1": {
            "features": features.tolist(),
            "phi": phi.tolist(),
            "init_labels": init_labels.tolist(),
            "k": 2,
            "fa": 1.0, "fb": 1.0,
            "gamma": gamma.tolist(),
            "pi": pi.tolist(),
        },
        "test2": {
            "features": features2.tolist(),
            "phi": phi.tolist(),
            "init_labels": init_labels2.tolist(),
            "k": 3,
            "fa": 1.0, "fb": 1.0,
            "gamma": gamma2.tolist(),
            "pi": pi2.tolist(),
        },
    }
    with open("vbx_reference.json", "w") as f:
        json.dump(ref_data, f, indent=2)
    print(f"\nReference data written to vbx_reference.json")


if __name__ == "__main__":
    main()
