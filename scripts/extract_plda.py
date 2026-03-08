#!/usr/bin/env python3
"""Extract PLDA parameters from pyannote's WeSpeaker model for Sumi's VBx clustering.

Usage:
    pip install pyannote.audio torch
    python scripts/extract_plda.py [output_path]

Requires a HuggingFace token with access to pyannote models.
Set HF_TOKEN env var or use `huggingface-cli login`.

Produces a binary file (default: plda.bin) with the pre-computed PLDA transform
parameters in Sumi's SUMIPLDA format.
"""

import sys
import struct
import numpy as np


def main():
    output_path = sys.argv[1] if len(sys.argv) > 1 else "plda.bin"

    try:
        from pyannote.audio.utils.vbx import vbx_setup
        from huggingface_hub import hf_hub_download
    except ImportError:
        print("Error: pyannote.audio not installed. Run: pip install pyannote.audio torch")
        sys.exit(1)

    import os
    token = os.environ.get("HF_TOKEN", None)
    if not token:
        print("Warning: HF_TOKEN not set. May fail for gated models.")

    repo_id = "pyannote/speaker-diarization-community-1"
    subfolder = "plda"
    print(f"Downloading PLDA files from {repo_id}/{subfolder} ...")

    xvec_path = hf_hub_download(repo_id, "xvec_transform.npz", subfolder=subfolder, token=token)
    plda_path = hf_hub_download(repo_id, "plda.npz", subfolder=subfolder, token=token)
    print(f"  xvec_transform: {xvec_path}")
    print(f"  plda:           {plda_path}")

    # Load raw npz files
    xvec_data = np.load(xvec_path)
    plda_data = np.load(plda_path)

    mean1 = xvec_data["mean1"].astype(np.float32).ravel()
    mean2 = xvec_data["mean2"].astype(np.float32).ravel()
    lda = xvec_data["lda"].astype(np.float32)  # (d_in, d_out)

    d_in = lda.shape[0]
    d_out = lda.shape[1]

    print(f"  d_in={d_in}, d_out={d_out}")
    print(f"  mean1: shape={mean1.shape}, range=[{mean1.min():.4f}, {mean1.max():.4f}]")
    print(f"  mean2: shape={mean2.shape}, range=[{mean2.min():.4f}, {mean2.max():.4f}]")
    print(f"  lda:   shape={lda.shape}")

    # Run vbx_setup to get the pre-computed PLDA transform
    xvec_tf, plda_tf, plda_psi_full = vbx_setup(xvec_path, plda_path)

    # Extract PLDA params from npz
    mu = plda_data["mu"].astype(np.float32).ravel()
    tr = plda_data["tr"].astype(np.float64)
    psi = plda_data["psi"].astype(np.float64).ravel()

    # Compute eigenvectors (same as vbx_setup)
    from scipy.linalg import eigh

    W = np.linalg.inv(tr.T @ tr)
    B = np.linalg.inv((tr.T / psi) @ tr)
    acvar, wccn = eigh(B, W)
    plda_psi = acvar[::-1].astype(np.float32)
    plda_tr_mat = wccn.T[::-1].astype(np.float32)

    plda_mu = mu
    phi = plda_psi[:d_out]

    print(f"  plda_mu: shape={plda_mu.shape}")
    print(f"  plda_tr: shape={plda_tr_mat.shape}")
    print(f"  phi:     shape={phi.shape}, range=[{phi.min():.6f}, {phi.max():.6f}]")

    # Verify our phi matches vbx_setup's
    np.testing.assert_allclose(phi, plda_psi_full[:d_out], atol=1e-5)
    print("  ✓ phi matches vbx_setup output")

    # Write binary file
    with open(output_path, "wb") as f:
        # Magic + version
        f.write(b"SUMIPLDA\x00")
        f.write(struct.pack("<B", 1))  # version

        # Dimensions
        f.write(struct.pack("<II", d_in, d_out))

        # Arrays
        f.write(mean1.tobytes())
        f.write(mean2.tobytes())
        f.write(lda.ravel().tobytes())  # row-major
        f.write(plda_mu.tobytes())
        f.write(plda_tr_mat[:d_out, :d_out].ravel().tobytes())  # row-major, truncated to d_out
        f.write(phi.tobytes())

    total_bytes = (
        9 + 1 + 8
        + d_in * 4
        + d_out * 4
        + d_in * d_out * 4
        + d_out * 4
        + d_out * d_out * 4
        + d_out * 4
    )
    print(f"\nWrote {output_path} ({total_bytes:,} bytes)")
    print(f"Copy to: ~/.sumi/models/plda.bin (or ~/.sumi-dev/models/plda.bin for dev)")

    # Verification: transform a test embedding and print
    print("\n--- Verification ---")
    test_emb = np.random.randn(d_in).astype(np.float32)
    # Python transform
    x = test_emb - mean1
    x = x / np.linalg.norm(x)
    x = np.sqrt(d_in) * x
    x = x @ lda
    x = x - mean2
    x = x / np.linalg.norm(x)
    x = np.sqrt(d_out) * x
    x = (x - plda_mu) @ plda_tr_mat[:d_out, :d_out].T
    print(f"Test embedding (first 8 dims): {x[:8]}")
    print("Use this to verify Rust transform matches.")


if __name__ == "__main__":
    main()
