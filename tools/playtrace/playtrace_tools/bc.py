"""A tiny numpy behaviour-cloning smoke over a human stream.

This proves the path ``dataset -> loader -> train -> validate`` end to end. It
is **not** a policy-quality claim: a few minutes of one player is far too little
to learn a usable policy, and the numbers below only say how this small model
fits that data on sessions it did not train on.

Inputs are the policy observation (ObservationV2) and, optionally, the previous
``history`` policy actions of the same trajectory. Nothing else is read: no
diagnostics, no hashes, no tick ids, no future observation.

Model: one hidden ReLU layer, a 3-way softmax for ``horizontal`` and sigmoid
heads for the five buttons. Buttons whose mask is 0 on a row are left out of the
loss on that row (they are not selectable there). Adam, full determinism from a
seed.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

import numpy as np

from . import schema
from .dataset import Dataset, DatasetError, Stream, split_rows

CAVEAT = (
    "BC smoke: this proves the loader/train/validation pipeline on a small amount of "
    "recorded play from few sessions; it does not measure or imply policy quality."
)
BUTTONS = schema.ACTION_COMPONENTS[1:]


@dataclass
class TrainConfig:
    hidden: int = 64
    epochs: int = 40
    batch: int = 256
    lr: float = 3e-3
    weight_decay: float = 1e-4
    history: int = 0
    seed: int = 0
    stream: str = "human"


def features(stream: Stream, index: np.ndarray, history: int) -> np.ndarray:
    """Policy features of the selected rows: the observation (+ previous actions)."""
    x = stream.obs[index].astype(np.float32)
    if history:
        if stream.context is None or stream.context.shape[1] < history:
            raise DatasetError(f"the dataset has no action context of length {history}: re-export with --context")
        ctx = stream.context[index][:, -history:, :].astype(np.float32)
        # The export zero-pads missing earlier actions, and 0 means "left": a validity
        # bit per slot tells padding from a real previous action (padding is at the front).
        have = np.array([min(stream.rows[i].get("ctx_len", 0), history) for i in index])
        valid = (np.arange(history)[None, :] >= (history - have)[:, None]).astype(np.float32)
        ctx = ctx * valid[:, :, None]
        x = np.concatenate([x, ctx.reshape(len(index), -1), valid], axis=1)
    return x


def _targets(stream: Stream, index: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    action = stream.action[index]
    return (
        action[:, 0].astype(np.int64),
        action[:, 1:].astype(np.float32),
        stream.mask[index][:, 1:].astype(np.float32),
    )


class Model:
    def __init__(self, n_in: int, config: TrainConfig) -> None:
        rng = np.random.default_rng(config.seed)
        self.params = {
            "w1": (rng.standard_normal((n_in, config.hidden)) * np.sqrt(2.0 / n_in)).astype(np.float32),
            "b1": np.zeros(config.hidden, np.float32),
            "wh": (rng.standard_normal((config.hidden, 3)) * 0.05).astype(np.float32),
            "bh": np.zeros(3, np.float32),
            "wb": (rng.standard_normal((config.hidden, len(BUTTONS))) * 0.05).astype(np.float32),
            "bb": np.zeros(len(BUTTONS), np.float32),
        }

    def forward(self, x: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
        p = self.params
        hidden = np.maximum(x @ p["w1"] + p["b1"], 0.0)
        return hidden, hidden @ p["wh"] + p["bh"], hidden @ p["wb"] + p["bb"]

    def predict(self, x: np.ndarray, mask: np.ndarray) -> np.ndarray:
        """Greedy legal action: argmax horizontal, buttons by p >= 0.5 where the mask allows."""
        _, hz, bt = self.forward(x)
        buttons = (bt >= 0.0) & (mask[:, 1:] > 0)
        return np.concatenate([hz.argmax(axis=1)[:, None], buttons.astype(np.int64)], axis=1)


def _losses(model: Model, x, horiz, buttons, bmask) -> tuple[float, float]:
    _, hz, bt = model.forward(x)
    hz = hz - hz.max(axis=1, keepdims=True)
    logp = hz - np.log(np.exp(hz).sum(axis=1, keepdims=True))
    ce = -logp[np.arange(len(horiz)), horiz].mean()
    bce = np.maximum(bt, 0) - bt * buttons + np.log1p(np.exp(-np.abs(bt)))
    return float(ce), float((bce * bmask).sum() / max(bmask.sum(), 1.0))


def _step(model: Model, adam: dict, x, horiz, buttons, bmask, config: TrainConfig, t: int) -> None:
    p = model.params
    hidden, hz, bt = model.forward(x)
    n = len(x)
    hz = hz - hz.max(axis=1, keepdims=True)
    prob = np.exp(hz)
    prob /= prob.sum(axis=1, keepdims=True)
    d_hz = prob
    d_hz[np.arange(n), horiz] -= 1.0
    d_hz /= n
    sig = 1.0 / (1.0 + np.exp(-bt))
    d_bt = (sig - buttons) * bmask / max(bmask.sum(), 1.0)
    d_hidden = d_hz @ p["wh"].T + d_bt @ p["wb"].T
    d_hidden[hidden <= 0.0] = 0.0
    grads = {
        "wh": hidden.T @ d_hz,
        "bh": d_hz.sum(axis=0),
        "wb": hidden.T @ d_bt,
        "bb": d_bt.sum(axis=0),
        "w1": x.T @ d_hidden + config.weight_decay * p["w1"],
        "b1": d_hidden.sum(axis=0),
    }
    for key, grad in grads.items():
        m, v = adam.setdefault(key, (np.zeros_like(grad), np.zeros_like(grad)))
        m = 0.9 * m + 0.1 * grad
        v = 0.999 * v + 0.001 * grad * grad
        adam[key] = (m, v)
        m_hat = m / (1 - 0.9**t)
        v_hat = v / (1 - 0.999**t)
        p[key] -= (config.lr * m_hat / (np.sqrt(v_hat) + 1e-8)).astype(np.float32)


def _prf(pred: np.ndarray, truth: np.ndarray, allowed: np.ndarray) -> dict[str, float | int]:
    pred = pred.astype(bool) & allowed.astype(bool)
    truth = truth.astype(bool) & allowed.astype(bool)
    tp = int((pred & truth).sum())
    fp = int((pred & ~truth).sum())
    fn = int((~pred & truth).sum())
    precision = tp / (tp + fp) if tp + fp else 0.0
    recall = tp / (tp + fn) if tp + fn else 0.0
    f1 = 2 * precision * recall / (precision + recall) if precision + recall else 0.0
    return {"positives": int(truth.sum()), "precision": precision, "recall": recall, "f1": f1}


def evaluate(model: Model, stream: Stream, index: np.ndarray, train_index: np.ndarray, history: int) -> dict[str, Any]:
    """Metrics on ``index`` plus the majority baseline learned on ``train_index``."""
    x = features(stream, index, history)
    horiz, buttons, bmask = _targets(stream, index)
    pred = model.predict(x, stream.mask[index])
    ce, bce = _losses(model, x, horiz, buttons, bmask)
    train_h = stream.action[train_index][:, 0]
    majority_h = int(np.bincount(train_h, minlength=3).argmax())
    out: dict[str, Any] = {
        "rows": int(len(index)),
        "horizontal_cross_entropy": ce,
        "button_bce": bce,
        "horizontal_accuracy": float((pred[:, 0] == horiz).mean()),
        "horizontal_majority_baseline_accuracy": float((horiz == majority_h).mean()),
        "buttons": {},
    }
    exact = pred[:, 0] == horiz
    for i, name in enumerate(BUTTONS):
        allowed = stream.mask[index][:, 1 + i]
        truth = buttons[:, i]
        p_i = pred[:, 1 + i]
        row = _prf(p_i, truth, allowed)
        row["accuracy"] = float(((p_i == truth) | (allowed == 0)).mean())
        row["never_pressed_baseline_accuracy"] = float(((truth == 0) | (allowed == 0)).mean())
        out["buttons"][name] = row
        exact &= (p_i == truth) | (allowed == 0)
    out["exact_action_accuracy"] = float(exact.mean())
    if stream.context is not None and stream.context.shape[1] >= 1:
        # The strongest trivial baseline for held inputs: repeat the previous action.
        valid = np.array([stream.rows[i].get("ctx_len", 0) > 0 for i in index])
        if valid.any():
            last = stream.context[index][valid][:, -1, :].astype(np.int64)
            truth = stream.action[index][valid].astype(np.int64)
            allowed = stream.mask[index][valid]
            same = (last == truth) | (allowed == 0)
            out["repeat_previous_action_baseline"] = {
                "rows": int(valid.sum()),
                "horizontal_accuracy": float((last[:, 0] == truth[:, 0]).mean()),
                "exact_action_accuracy": float(same.all(axis=1).mean()),
            }
    return out


def train_bc(dataset: Dataset, config: TrainConfig) -> dict[str, Any]:
    if config.stream not in dataset.streams:
        raise DatasetError(f"the dataset has no {config.stream!r} stream")
    if config.stream != "human":
        raise DatasetError("behaviour cloning trains on the human stream only")
    stream = dataset.streams[config.stream]
    train_idx = split_rows(stream, "train")
    dev_idx = split_rows(stream, "dev")
    holdout_idx = split_rows(stream, "holdout")
    if len(train_idx) == 0:
        raise DatasetError("no training rows")
    if len(dev_idx) == 0:
        raise DatasetError(
            "no validation rows: a leak-free validation split needs at least two sessions "
            "(split is by session; ticks of one session are never split)"
        )
    x_train = features(stream, train_idx, config.history)
    horiz, buttons, bmask = _targets(stream, train_idx)
    model = Model(x_train.shape[1], config)
    rng = np.random.default_rng(config.seed + 1)
    adam: dict = {}
    t = 0
    history_curve = []
    for epoch in range(config.epochs):
        order = rng.permutation(len(train_idx))
        for start in range(0, len(order), config.batch):
            batch = order[start : start + config.batch]
            t += 1
            _step(model, adam, x_train[batch], horiz[batch], buttons[batch], bmask[batch], config, t)
        if epoch in (0, config.epochs - 1) or (epoch + 1) % 10 == 0:
            ce, bce = _losses(model, x_train, horiz, buttons, bmask)
            dev_ce, dev_bce = _losses(model, features(stream, dev_idx, config.history), *_targets(stream, dev_idx))
            history_curve.append(
                {"epoch": epoch + 1, "train_ce": ce, "train_bce": bce, "dev_ce": dev_ce, "dev_bce": dev_bce}
            )
    report = {
        "dataset_id": dataset.meta["dataset_id"],
        "synthetic": dataset.meta["synthetic"],
        "caveat": CAVEAT,
        "config": config.__dict__,
        "inputs": "ObservationV2 (policy observation)" + (f" + previous {config.history} actions" if config.history else ""),
        "feature_width": int(x_train.shape[1]),
        "rows": {"train": int(len(train_idx)), "dev": int(len(dev_idx)), "holdout": int(len(holdout_idx))},
        "sessions_by_split": {
            s: len(g) for s, g in dataset.meta["leakage"]["groups_by_split"].items()
        },
        "curve": history_curve,
        "train": evaluate(model, stream, train_idx, train_idx, config.history),
        "dev": evaluate(model, stream, dev_idx, train_idx, config.history),
    }
    if len(holdout_idx):
        report["holdout"] = evaluate(model, stream, holdout_idx, train_idx, config.history)
    if dataset.meta["synthetic"]:
        report["caveat"] += " The data is a SYNTHETIC fixture, not a human demonstration."
    return report
