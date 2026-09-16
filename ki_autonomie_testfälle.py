#!/usr/bin/env python3
"""Repository test orchestrator for scientifically constrained AI changes.

The executable tests live in ``src/scientific_validation.rs`` because the
simulation is implemented in Rust. This dependency-free Python entry point
turns those tests into an explicit finish line for automated changes.

Default validation is deterministic and hardware-independent. Timing checks
are opt-in because an absolute runtime limit is only meaningful after it has
been calibrated on the target machine.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parent

SCIENTIFIC_TARGETS: dict[str, tuple[str, ...]] = {
    "unit": (
        "Plummer-softened gravity matches the analytic reference",
        "Unsoftened gravity follows the inverse-square law",
        "Zero-distance self-force remains finite and zero",
        "Octree mass and center of mass are exact",
        "Body integration remains semi-implicit Euler",
    ),
    "physics": (
        "Linear momentum is conserved in an isolated binary",
        "Angular-momentum drift remains below the numerical tolerance",
        "Symplectic energy drift remains bounded",
        "The center of mass remains stationary",
        "Full production steps conserve mass when no collision occurs",
        "Merging conserves mass and linear momentum",
        "A default-adaptive production binary remains bounded",
        "Disk equilibrium uses the runtime Plummer circular-speed law",
        "Legacy inclined generation uses projected orbital radius",
        "Elemental mass remapping recomputes circular velocities",
    ),
    "property": (
        "Exact tree traversal agrees with direct summation",
        "Pair forces obey Newton's third law",
        "Barnes-Hut RMS force error remains below five percent at theta=0.5",
        "Adaptive mixed-precision force orders remain below eight percent RMS error",
        "Simulation.attract remains below eight percent RMS error",
        "Fusion pressure is restricted to the local body neighborhood",
        "Elemental gravity tuning cannot create self-pressure",
        "Gravity is translation invariant",
        "Gravity is rotation covariant",
        "Bounded inputs never produce NaN or infinity",
    ),
    "shader": (
        "The optional GLSL artifact parses and validates with Naga",
        "The shipped CPU gas-billboard alpha layers are bounded and ordered",
    ),
    "regression": (
        "Softened force matches an independently calculated snapshot",
        "An equal-mass binary remains on a bounded circular orbit",
    ),
    "performance": (
        "Body memory remains within the 192-byte repository budget",
        "Octree storage growth remains linear",
        "Optional 4096-body timing stays within a calibrated machine budget",
        "Optional 4x particle scaling remains below 12x runtime growth",
    ),
}


def cargo_executable() -> str:
    cargo = shutil.which("cargo")
    if cargo is None:
        raise RuntimeError("Cargo was not found on PATH.")
    return cargo


def run(command: list[str], env: dict[str, str] | None = None) -> None:
    print("+", subprocess.list2cmdline(command), flush=True)
    completed = subprocess.run(command, cwd=ROOT, env=env, check=False)
    if completed.returncode != 0:
        raise SystemExit(completed.returncode)


def validate_category_contract() -> None:
    command = [
        cargo_executable(),
        "test",
        "--locked",
        "scientific_validation::",
        "--",
        "--list",
    ]
    completed = subprocess.run(
        command,
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.returncode != 0:
        sys.stderr.write(completed.stdout)
        sys.stderr.write(completed.stderr)
        raise SystemExit(completed.returncode)

    discovered = [
        line.removesuffix(": test")
        for line in completed.stdout.splitlines()
        if line.startswith("scientific_validation::") and line.endswith(": test")
    ]
    for category, targets in SCIENTIFIC_TARGETS.items():
        prefix = f"scientific_validation::{category}_"
        actual_count = sum(name.startswith(prefix) for name in discovered)
        if actual_count != len(targets):
            raise SystemExit(
                f"{category} documents {len(targets)} targets but discovers "
                f"{actual_count} Rust tests"
            )


def run_deterministic_suite(category: str) -> None:
    command = [cargo_executable(), "test", "--locked"]
    if category != "all":
        command.append(f"scientific_validation::{category}_")
        command.extend(["--", "--nocapture"])
    run(command)


def run_performance_gate(budget_ms: int) -> None:
    env = os.environ.copy()
    env["SCIENTIFIC_PERFORMANCE_BUDGET_MS"] = str(budget_ms)
    run(
        [
            cargo_executable(),
            "test",
            "--release",
            "--locked",
            "scientific_validation::performance_",
            "--",
            "--ignored",
            "--nocapture",
        ],
        env=env,
    )


def print_targets() -> None:
    for category, targets in SCIENTIFIC_TARGETS.items():
        print(f"{category}:")
        for target in targets:
            print(f"  - {target}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the scientific finish line for repository changes."
    )
    parser.add_argument(
        "--category",
        choices=("all", *SCIENTIFIC_TARGETS),
        default="all",
        help="Run one scientific category or the complete deterministic suite.",
    )
    parser.add_argument(
        "--performance",
        action="store_true",
        help="Also run the ignored release-mode hardware timing gate.",
    )
    parser.add_argument(
        "--budget-ms",
        type=int,
        default=250,
        help="Calibrated limit for the optional 4096-body timing gate.",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="List all enforced scientific targets without running tests.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    validate_category_contract()
    if args.list:
        print_targets()
        return 0
    if args.budget_ms <= 0:
        raise SystemExit("--budget-ms must be greater than zero")
    if not (ROOT / "Cargo.toml").is_file():
        raise SystemExit(f"Cargo.toml not found in {ROOT}")

    run_deterministic_suite(args.category)
    if args.performance or args.category == "performance":
        run_performance_gate(args.budget_ms)
    return 0


if __name__ == "__main__":
    sys.exit(main())
