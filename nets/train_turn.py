"""Trains the turn value net: (board, stack-to-pot ratio, both ranges) at the
start of the turn -> every hand's value for both players, in pot units.

Data comes from `datagen` via `datagen-export` (see rust/src/bin/datagen_export.rs):
flat arrays with hands in the canonical 1326 order. The whole set is moved to
the GPU once, so batching is pure on-device indexing.

- Inputs: both ranges (normalized to sum 1, scaled by 1326), the 4 board cards
  one-hot, SPR features, and range summaries over the per-hand strength features
  (`hand_features` in rust/src/spots.rs). Values are in pot units and the game is
  scale-free in chips, so the pot itself is not an input.
- Architectures: `mlp` (one output per hand from a shared trunk) or `hand` (a
  per-hand head shared across all hands: trunk embedding + the hand's strength
  features, its cards, and its strength rank against the opponent's range).
- Augmentation: a random suit permutation per sample (24 isomorphic copies).
- Output: an exact zero-sum correction with card removal (the two players'
  range-weighted values over compatible pairs must cancel).
- Loss: Huber on the hands inside each player's range.

Usage: python nets/train_turn.py --data <export dir> [<export dir> ...] --out <run dir>
"""

import argparse
import itertools
import json
import math
import os
import time

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F

N_HANDS = 1326


def canonical_hands():
    pairs = [(a, b) for a in range(52) for b in range(a + 1, 52)]
    index = {p: i for i, p in enumerate(pairs)}
    return pairs, index


def suit_tables(device):
    """Gather tables for the 24 suit permutations: permuted[x] = original[table[x]]."""
    pairs, index = canonical_hands()
    hand_tables, card_maps = [], []
    for perm in itertools.permutations(range(4)):
        f = lambda c: 4 * (c // 4) + perm[c % 4]
        t = [0] * N_HANDS
        for i, (a, b) in enumerate(pairs):
            x, y = sorted((f(a), f(b)))
            t[index[(x, y)]] = i
        hand_tables.append(t)
        card_maps.append([f(c) for c in range(52)])
    return (torch.tensor(hand_tables, device=device), torch.tensor(card_maps, device=device))


def incidence(device):
    """1326 x 52 hand-card incidence matrix."""
    pairs, _ = canonical_hands()
    c = torch.zeros(N_HANDS, 52, device=device)
    for i, (a, b) in enumerate(pairs):
        c[i, a] = 1
        c[i, b] = 1
    return c


def load(dirs):
    parts = {k: [] for k in ["boards", "scalars", "index", "ranges", "values", "feats"]}
    for d in dirs:
        meta = json.load(open(os.path.join(d, "meta.json")))
        n, nf = meta["n"], meta["n_features"]
        parts["boards"].append(np.fromfile(os.path.join(d, "boards.u8"), np.uint8).reshape(n, 4))
        parts["scalars"].append(np.fromfile(os.path.join(d, "scalars.f32"), np.float32).reshape(n, 4))
        parts["index"].append(np.fromfile(os.path.join(d, "index.u64"), np.uint64))
        parts["ranges"].append(np.fromfile(os.path.join(d, "ranges.f16"), np.float16).reshape(n, 2, N_HANDS))
        parts["values"].append(np.fromfile(os.path.join(d, "values.f16"), np.float16).reshape(n, 2, N_HANDS))
        parts["feats"].append(np.fromfile(os.path.join(d, "feats.f16"), np.float16).reshape(n, N_HANDS, nf))
    return {k: np.concatenate(v) for k, v in parts.items()}


def spr_features(spr):
    return torch.stack([torch.log(spr), spr / (1 + spr), 1 / (1 + spr), torch.sqrt(spr) / 8], dim=1)


N_FEATS = 10
N_SUMMARY = 2 * (N_FEATS + 10)


def card_pairs(device):
    pairs, _ = canonical_hands()
    return torch.tensor(pairs, device=device)


class Trunk(nn.Module):
    def __init__(self, hidden, blocks):
        super().__init__()
        self.inp = nn.Linear(2 * N_HANDS + 52 + 4 + N_SUMMARY, hidden)
        self.blocks = nn.ModuleList(
            nn.Sequential(nn.LayerNorm(hidden), nn.Linear(hidden, hidden), nn.GELU(), nn.Linear(hidden, hidden))
            for _ in range(blocks)
        )
        self.norm = nn.LayerNorm(hidden)

    def forward(self, x):
        h = self.inp(x)
        for b in self.blocks:
            h = h + b(h)
        return self.norm(h)


class ValueNet(nn.Module):
    """arch "mlp": trunk -> 2 x 1326 outputs."""

    def __init__(self, hidden=1536, blocks=4, **_):
        super().__init__()
        self.trunk = Trunk(hidden, blocks)
        self.out = nn.Linear(hidden, 2 * N_HANDS)

    def forward(self, x, hand_x):
        return self.out(self.trunk(x)).view(-1, 2, N_HANDS)


class HandNet(nn.Module):
    """arch "hand": trunk embedding combined with each hand's own inputs in a
    head shared across hands (one head per player)."""

    def __init__(self, hidden=1536, blocks=4, head=256, **_):
        super().__init__()
        self.trunk = Trunk(hidden, blocks)
        self.ctx = nn.Linear(hidden, 2 * head)
        self.hand = nn.ModuleList(nn.Linear(N_FEATS + 1, head) for _ in range(2))
        self.cards = nn.Parameter(torch.randn(2, 52, head) * 0.02)
        self.mlp = nn.ModuleList(
            nn.Sequential(nn.GELU(), nn.Linear(head, head), nn.GELU(), nn.Linear(head, 1)) for _ in range(2)
        )
        self.register_buffer("pairs", card_pairs("cpu"), persistent=False)
        self.head = head

    def forward(self, x, hand_x):
        # hand_x: (B, 2, 1326, N_FEATS + 1) per-player hand inputs
        b = x.shape[0]
        ctx = self.ctx(self.trunk(x)).view(b, 2, 1, self.head)
        out = []
        for p in range(2):
            cards = self.cards[p][self.pairs].sum(1)  # (1326, head)
            z = ctx[:, p] + self.hand[p](hand_x[:, p]) + cards
            out.append(self.mlp[p](z).squeeze(-1))
        return torch.stack(out, 1)


class Batcher:
    """Builds net inputs and targets on the device, with suit augmentation."""

    def __init__(self, data, device):
        self.device = device
        self.boards = torch.from_numpy(data["boards"].astype(np.int64)).to(device)
        sc = torch.from_numpy(data["scalars"]).to(device)
        self.spr = sc[:, 1] / sc[:, 0]
        self.ranges = torch.from_numpy(data["ranges"]).to(device)
        self.values = torch.from_numpy(data["values"]).to(device)
        self.feats = torch.from_numpy(data["feats"]).to(device)
        self.hand_tables, self.card_maps = suit_tables(device)
        self.inc = incidence(device)

    def __call__(self, idx, augment):
        b = len(idx)
        r = self.ranges[idx].float()
        v = self.values[idx].float()
        f = self.feats[idx].float()
        cards = self.boards[idx]
        if augment:
            k = torch.randint(0, 24, (b,), device=self.device)
            t = self.hand_tables[k][:, None, :].expand(b, 2, N_HANDS)
            r = torch.gather(r, 2, t)
            v = torch.gather(v, 2, t)
            f = torch.gather(f, 1, self.hand_tables[k][:, :, None].expand(b, N_HANDS, f.shape[2]))
            cards = torch.gather(self.card_maps[k], 1, cards)
        r = r / r.sum(dim=2, keepdim=True)
        board = torch.zeros(b, 52, device=self.device).scatter_(1, cards, 1.0)
        # range summaries: mean strength features, and mass over strength deciles
        summ = [torch.einsum("bph,bhf->bpf", r, f).reshape(b, -1)]
        dec = (f[:, :, 0] * 10).long().clamp(0, 9)
        summ.append(torch.zeros(b, 2, 10, device=self.device).scatter_add_(2, dec[:, None, :].expand(b, 2, N_HANDS), r).reshape(b, -1))
        x = torch.cat([r[:, 0] * N_HANDS, r[:, 1] * N_HANDS, board, spr_features(self.spr[idx])] + summ, dim=1)
        # per hand: its strength features plus its strength rank against the
        # opponent's range (by mean percentile, ties half, card removal ignored)
        strength = f[:, :, 0]
        order = strength.argsort(dim=1)
        opp = r.flip(1)
        sorted_mass = torch.gather(opp, 2, order[:, None, :].expand(b, 2, N_HANDS))
        below_sorted = sorted_mass.cumsum(2) - 0.5 * sorted_mass
        rank = torch.empty_like(below_sorted).scatter_(2, order[:, None, :].expand(b, 2, N_HANDS), below_sorted)
        hand_x = torch.cat([f[:, None].expand(b, 2, N_HANDS, f.shape[2]), rank[..., None]], dim=3)
        mask = torch.isfinite(v) & (r > 0)
        return x, hand_x, r, torch.where(mask, v, torch.zeros_like(v)), mask

    def pair_weights(self, r):
        """w[p, i] = r_p[i] * (opponent mass compatible with hand i)."""
        opp = r.flip(1)
        per_card = opp @ self.inc  # (B, 2, 52)
        m = opp.sum(2, keepdim=True) - per_card @ self.inc.T + opp
        return r * m


def zero_sum(pred, w):
    """Shifts both players' values by the same amount so the weighted sum cancels."""
    total = w[:, 0].sum(1, keepdim=True)
    err = (w * pred).sum(dim=(1, 2)).unsqueeze(1) / total
    return pred - 0.5 * err.unsqueeze(2)


def evaluate(model, batcher, idx, bs, spr_all):
    model.eval()
    stats = {}
    with torch.no_grad():
        for s in range(0, len(idx), bs):
            j = idx[s : s + bs]
            x, hand_x, r, y, mask = batcher(j, augment=False)
            with torch.autocast("cuda", dtype=torch.bfloat16, enabled=batcher.device.type == "cuda"):
                pred = model(x, hand_x)
            pred = zero_sum(pred.float(), batcher.pair_weights(r))
            err = (pred - y).abs() * mask
            w = batcher.pair_weights(r) * mask
            spr = spr_all[j]
            buckets = {"all": torch.ones_like(spr, dtype=torch.bool), "spr<1": spr < 1, "spr1-4": (spr >= 1) & (spr < 4),
                       "spr4-15": (spr >= 4) & (spr < 15), "spr>=15": spr >= 15}
            for name, sel in buckets.items():
                a = stats.setdefault(name, [0.0, 0.0, 0.0, 0.0])
                a[0] += err[sel].sum().item()
                a[1] += mask[sel].sum().item()
                a[2] += (err * w)[sel].sum().item()
                a[3] += w[sel].sum().item()
    model.train()
    # mean absolute error per in-range hand, and range-weighted (pot units)
    return {k: (a[0] / max(a[1], 1), a[2] / max(a[3], 1e-12)) for k, a in stats.items()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", nargs="+", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--epochs", type=float, default=50)
    ap.add_argument("--batch", type=int, default=1024)
    ap.add_argument("--lr", type=float, default=1e-3)
    ap.add_argument("--wd", type=float, default=1e-4)
    ap.add_argument("--hidden", type=int, default=1536)
    ap.add_argument("--blocks", type=int, default=4)
    ap.add_argument("--arch", choices=["mlp", "hand"], default="hand")
    ap.add_argument("--head", type=int, default=256)
    ap.add_argument("--huber", type=float, default=1.0)
    ap.add_argument("--val-mod", type=int, default=50, help="spot index %% val_mod == 0 -> validation")
    ap.add_argument("--evals", type=int, default=50, help="evaluations over the run")
    ap.add_argument("--limit", type=int, default=0)
    args = ap.parse_args()

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    torch.backends.cuda.matmul.allow_tf32 = True
    torch.backends.cudnn.allow_tf32 = True
    os.makedirs(args.out, exist_ok=True)
    t0 = time.time()
    data = load(args.data)
    if args.limit:
        data = {k: v[: args.limit] for k, v in data.items()}
    n = len(data["index"])
    is_val = (data["index"] % args.val_mod) == 0
    train_idx = torch.from_numpy(np.nonzero(~is_val)[0]).to(device)
    val_idx = torch.from_numpy(np.nonzero(is_val)[0]).to(device)
    batcher = Batcher(data, device)
    print(f"{n} samples ({len(train_idx)} train, {len(val_idx)} val) on {device}, loaded in {time.time() - t0:.0f} s", flush=True)

    net = {"mlp": ValueNet, "hand": HandNet}[args.arch]
    model = net(hidden=args.hidden, blocks=args.blocks, head=args.head).to(device)
    print(f"{sum(p.numel() for p in model.parameters()) / 1e6:.1f}M parameters", flush=True)
    opt = torch.optim.AdamW(model.parameters(), lr=args.lr, weight_decay=args.wd)
    steps = int(args.epochs * len(train_idx) / args.batch)
    warmup = min(1000, steps // 20)
    sched = torch.optim.lr_scheduler.LambdaLR(
        opt, lambda s: min(1.0, (s + 1) / max(warmup, 1)) * 0.5 * (1 + math.cos(math.pi * min(s, steps) / steps))
    )
    eval_every = max(steps // args.evals, 1)
    log = open(os.path.join(args.out, "log.csv"), "a")
    log.write("step,epoch,seconds,train_loss,lr," + ",".join(f"{b}_{m}" for b in ["all", "spr<1", "spr1-4", "spr4-15", "spr>=15"] for m in ["mae", "wmae"]) + "\n")
    config = vars(args) | {"n": n, "steps": steps}
    json.dump(config, open(os.path.join(args.out, "config.json"), "w"), indent=2)

    best = float("inf")
    running = 0.0
    t0 = time.time()
    for step in range(steps):
        j = train_idx[torch.randint(0, len(train_idx), (args.batch,), device=device)]
        x, hand_x, r, y, mask = batcher(j, augment=True)
        with torch.autocast("cuda", dtype=torch.bfloat16, enabled=device.type == "cuda"):
            pred = model(x, hand_x)
        pred = zero_sum(pred.float(), batcher.pair_weights(r))
        loss = (F.huber_loss(pred, y, reduction="none", delta=args.huber) * mask).sum() / mask.sum()
        opt.zero_grad(set_to_none=True)
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
        opt.step()
        sched.step()
        running = 0.98 * running + 0.02 * loss.item() if step else loss.item()
        if (step + 1) % eval_every == 0 or step + 1 == steps:
            ev = evaluate(model, batcher, val_idx, 4096, batcher.spr)
            epoch = (step + 1) * args.batch / len(train_idx)
            el = time.time() - t0
            print(
                f"step {step + 1}/{steps} epoch {epoch:.2f} {el:.0f}s loss {running:.5f} lr {sched.get_last_lr()[0]:.2e} | val mae "
                + " ".join(f"{k} {v[0]:.4f}/{v[1]:.4f}" for k, v in ev.items()),
                flush=True,
            )
            log.write(f"{step + 1},{epoch:.3f},{el:.0f},{running:.6f},{sched.get_last_lr()[0]:.3e}," + ",".join(f"{v[0]:.6f},{v[1]:.6f}" for v in ev.values()) + "\n")
            log.flush()
            ckpt = {"model": model.state_dict(), "config": config, "step": step + 1, "val": ev}
            torch.save(ckpt, os.path.join(args.out, "last.pt"))
            if ev["all"][1] < best:
                best = ev["all"][1]
                torch.save(ckpt, os.path.join(args.out, "best.pt"))
    print(f"done: best val range-weighted mae {best:.5f} pot", flush=True)


if __name__ == "__main__":
    main()
