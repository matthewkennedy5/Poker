//! Variance-reduced evaluation of Slumbot match logs (the chance-node part of
//! AIVAT, plus all-in adjustment as a special case).
//!
//! Slumbot reveals its hole cards at the end of every hand, so for every chance
//! event that occurred (hole-card deal, flop, turn, river) we know both hands and
//! can add a zero-mean correction
//!
//!     term = E_c'[ v(after c') ] - v(after c)
//!
//! with v = (our equity vs Slumbot's actual hand - 1/2) * pot. Equity is a
//! martingale over the deal, so for the board cards the term is simply
//! (equity before - equity after) * pot. Each term has zero conditional mean
//! whatever v is, so the corrected per-hand value is an unbiased estimate of
//! the win rate. For the hole-card deal the pot multiplier is a constant K,
//! chosen per log by regression on the *other* logs (cross-fitting).
//!
//! "All-in only" applies just the terms after both players are all in; that is
//! exactly the classic all-in EV adjustment.
//!
//! Usage: aivat [--samples 20000] [--k <fixed K>] [--csv per_hand.csv] log1.jsonl ...
//! `--csv` also writes per-hand street attribution terms (see `street_terms`).
use optimus::Card;
use rand::prelude::*;
use rand::rngs::StdRng;
use rayon::prelude::*;
use rs_poker::core::{Card as RsCard, Rank, Rankable, Suit, Value};

const STACK: i64 = 20000;

struct Seven([RsCard; 7]);
impl Rankable for Seven {
    fn cards(&self) -> &[RsCard] {
        &self.0
    }
}

fn rs(c: &Card) -> RsCard {
    RsCard {
        value: Value::from_u8(c.rank - 2),
        suit: Suit::from_u8(c.suit),
    }
}

fn rank7(h: [RsCard; 2], board: &[RsCard; 5]) -> Rank {
    Seven([h[0], h[1], board[0], board[1], board[2], board[3], board[4]]).rank()
}

/// Showdown score for us on a complete board: 1, 0.5 or 0.
fn score(us: [RsCard; 2], them: [RsCard; 2], board: &[RsCard; 5]) -> f64 {
    let a = rank7(us, board);
    let b = rank7(them, board);
    if a > b {
        1.0
    } else if a == b {
        0.5
    } else {
        0.0
    }
}

fn remaining(dead: &[RsCard]) -> Vec<RsCard> {
    let mut out = Vec::with_capacity(52);
    for v in 0..13u8 {
        for s in 0..4u8 {
            let c = RsCard { value: Value::from_u8(v), suit: Suit::from_u8(s) };
            if !dead.contains(&c) {
                out.push(c);
            }
        }
    }
    out
}

/// Exact equity given `known` board cards (0..=5), enumerating the rest. Used
/// with 3+ known cards (and for preflop all-ins, with 0).
fn equity_exact(us: [RsCard; 2], them: [RsCard; 2], known: &[RsCard]) -> f64 {
    let mut dead = vec![us[0], us[1], them[0], them[1]];
    dead.extend_from_slice(known);
    let rest = remaining(&dead);
    let need = 5 - known.len();
    let mut board = [us[0]; 5];
    board[..known.len()].copy_from_slice(known);
    let mut total = 0.0;
    let mut count = 0u64;
    fn rec(
        rest: &[RsCard], start: usize, need: usize, pos: usize, board: &mut [RsCard; 5],
        us: [RsCard; 2], them: [RsCard; 2], total: &mut f64, count: &mut u64,
    ) {
        if need == 0 {
            *total += score(us, them, board);
            *count += 1;
            return;
        }
        for i in start..rest.len() {
            board[pos] = rest[i];
            rec(rest, i + 1, need - 1, pos + 1, board, us, them, total, count);
        }
    }
    rec(&rest, 0, need, known.len(), &mut board, us, them, &mut total, &mut count);
    total / count as f64
}

/// Unbiased Monte Carlo estimate of preflop equity.
fn equity_preflop_mc(us: [RsCard; 2], them: [RsCard; 2], samples: usize, seed: u64) -> f64 {
    let rest = remaining(&[us[0], us[1], them[0], them[1]]);
    let mut rng = StdRng::seed_from_u64(seed);
    let mut idx: Vec<usize> = (0..rest.len()).collect();
    let mut total = 0.0;
    for _ in 0..samples {
        // partial Fisher-Yates for 5 cards
        for k in 0..5 {
            let j = rng.gen_range(k..idx.len());
            idx.swap(k, j);
        }
        let board = [rest[idx[0]], rest[idx[1]], rest[idx[2]], rest[idx[3]], rest[idx[4]]];
        total += score(us, them, &board);
    }
    total / samples as f64
}

/// Pot at the end of each street (index 0 = after preflop), from Slumbot's
/// action string. Bets are street totals ("b200" = put 200 in on this street).
fn street_end_pots(action: &str) -> Vec<i64> {
    let mut total = [0i64; 2]; // [SB, BB] contributions before the current street
    let mut pots = Vec::new();
    for (street, s) in action.split('/').enumerate() {
        let mut bets: [i64; 2] = if street == 0 { [50, 100] } else { [0, 0] };
        let mut p = if street == 0 { 0 } else { 1 };
        let bytes = s.as_bytes();
        let mut i = 0;
        let mut folded = false;
        while i < bytes.len() {
            match bytes[i] {
                b'b' => {
                    let mut j = i + 1;
                    while j < bytes.len() && bytes[j].is_ascii_digit() {
                        j += 1;
                    }
                    bets[p] = s[i + 1..j].parse().unwrap();
                    i = j;
                }
                b'c' => {
                    bets[p] = bets[1 - p];
                    i += 1;
                }
                b'k' => i += 1,
                b'f' => {
                    folded = true;
                    i += 1;
                }
                _ => i += 1,
            }
            p = 1 - p;
        }
        total[0] += bets[0];
        total[1] += bets[1];
        if folded {
            break;
        }
        pots.push(total[0] + total[1]);
    }
    pots
}

/// Pot and per-seat contributions [SB, BB] at the end of each street's betting
/// (index 0 = preflop), and the street on which someone folded, if anyone did.
fn street_states(action: &str) -> (Vec<(i64, [i64; 2])>, Option<usize>) {
    let mut total = [0i64; 2];
    let mut states = Vec::new();
    for (street, s) in action.split('/').enumerate() {
        let mut bets: [i64; 2] = if street == 0 { [50, 100] } else { [0, 0] };
        let mut p = if street == 0 { 0 } else { 1 };
        let bytes = s.as_bytes();
        let mut i = 0;
        let mut folded = false;
        while i < bytes.len() {
            match bytes[i] {
                b'b' => {
                    let mut j = i + 1;
                    while j < bytes.len() && bytes[j].is_ascii_digit() {
                        j += 1;
                    }
                    bets[p] = s[i + 1..j].parse().unwrap();
                    i = j;
                }
                b'c' => {
                    bets[p] = bets[1 - p];
                    i += 1;
                }
                b'k' => i += 1,
                b'f' => {
                    folded = true;
                    i += 1;
                }
                _ => i += 1,
            }
            p = 1 - p;
        }
        if folded {
            return (states, Some(street));
        }
        total[0] += bets[0];
        total[1] += bets[1];
        states.push((total[0] + total[1], total));
    }
    (states, None)
}

/// Street attribution: with W = equity * pot - our chips in (our showdown share
/// with no more betting), raw = W(blinds) + sum of chance moves (zero mean)
/// + sum over streets of the W change caused by that street's betting. The
/// betting terms are zero-sum, so averaged over both seats they are exactly 0
/// when both players use the same strategy.
fn street_terms(raw: f64, seat: usize, eqs: &[f64], action: &str) -> [f64; 4] {
    let (states, fold_street) = street_states(action);
    let mut out = [0.0; 4];
    let blinds = [50i64, 100];
    let mut prev_pot = 150i64;
    let mut prev_c = blinds[seat];
    for k in 0..4 {
        let eq = match eqs.get(k) {
            Some(&e) => e,
            None => break,
        };
        let w_start = eq * prev_pot as f64 - prev_c as f64;
        if fold_street == Some(k) {
            out[k] = raw - w_start;
            break;
        }
        let Some(&(pot, c)) = states.get(k) else { break };
        out[k] = eq * pot as f64 - c[seat] as f64 - w_start;
        prev_pot = pot;
        prev_c = c[seat];
    }
    out
}

struct HandRes {
    raw: f64,
    allin: f64,       // raw + post-all-in chance terms (classic all-in EV)
    post: f64,        // raw + all flop/turn/river chance terms
    x: f64,           // eq_preflop - 0.5 (regressor for the deal term)
    act: f64,         // sum of our-decision terms: E_sigma[v] - v(sampled)
    n_act: usize,     // decisions that had values
    seat: usize,      // 0 = SB, 1 = BB
    streets: [f64; 4],
    action: String,
}

/// Our-decision AIVAT terms from the logged sampling distribution and values.
fn action_terms(rec: &serde_json::Value) -> (f64, usize) {
    let mut total = 0.0;
    let mut n = 0;
    let decisions = match rec.get("decisions").and_then(|d| d.as_array()) {
        Some(d) => d,
        None => return (0.0, 0),
    };
    for d in decisions {
        let (probs, values, sampled) = match (
            d.get("probs").and_then(|p| p.as_array()),
            d.get("values").and_then(|v| v.as_array()),
            d.get("sampled"),
        ) {
            (Some(p), Some(v), Some(s)) if !p.is_empty() && p.len() == v.len() => (p, v, s),
            _ => continue,
        };
        let mut expected = 0.0;
        let mut taken: Option<f64> = None;
        let mut psum = 0.0;
        for (pv, vv) in probs.iter().zip(values.iter()) {
            let (a, p) = match pv.as_array() {
                Some(t) if t.len() == 2 => (&t[0], t[1].as_f64().unwrap_or(0.0)),
                _ => continue,
            };
            let v = vv.as_f64().unwrap_or(0.0);
            expected += p * v;
            psum += p;
            if a == sampled {
                taken = Some(v);
            }
        }
        if let Some(v) = taken {
            if (psum - 1.0).abs() < 1e-6 {
                total += expected - v;
                n += 1;
            }
        }
    }
    (total, n)
}

fn evaluate(rec: &serde_json::Value, samples: usize, seed: u64) -> Option<HandRes> {
    let raw = rec.get("winnings")?.as_f64()?;
    let parse = |key: &str| -> Option<Vec<Card>> {
        rec.get(key)?.as_array()?.iter().map(|c| c.as_str().map(Card::new)).collect()
    };
    let us = parse("hole_cards")?;
    let them = parse("bot_hole_cards")?;
    let board = parse("board")?;
    let action = rec.get("action")?.as_str()?;
    let us = [rs(&us[0]), rs(&us[1])];
    let them = [rs(&them[0]), rs(&them[1])];
    let board: Vec<RsCard> = board.iter().map(rs).collect();
    let pots = street_end_pots(action);

    // Equity before each chance event that occurred: index 0 = preflop.
    let preflop_allin = pots.first().map_or(false, |&p| p >= 2 * STACK);
    let eq_pre = if preflop_allin {
        equity_exact(us, them, &[])
    } else {
        equity_preflop_mc(us, them, samples, seed)
    };
    let mut eqs = vec![eq_pre];
    for n in [3usize, 4, 5] {
        if board.len() >= n && pots.len() >= n - 2 {
            eqs.push(equity_exact(us, them, &board[..n]));
        }
    }
    let mut post = raw;
    let mut allin = raw;
    for k in 1..eqs.len() {
        let pot = pots[k - 1] as f64; // pot when card(s) k are dealt
        let term = (eqs[k - 1] - eqs[k]) * pot;
        post += term;
        if pots[k - 1] >= 2 * STACK {
            allin += term;
        }
    }
    let (act, n_act) = action_terms(rec);
    // Slumbot's client_pos 0 means we are the big blind.
    let seat = if rec.get("client_pos")?.as_i64()? == 0 { 1 } else { 0 };
    let streets = street_terms(raw, seat, &eqs, action);
    Some(HandRes { raw, allin, post, x: eq_pre - 0.5, act, n_act, seat, streets, action: action.to_string() })
}

fn mean_ci(v: &[f64]) -> (f64, f64, f64) {
    let n = v.len() as f64;
    let m = v.iter().sum::<f64>() / n;
    let var = v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0);
    (m, 1.96 * (var / n).sqrt(), var.sqrt())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut samples = 20000usize;
    let mut fixed_k: Option<f64> = None;
    let mut csv: Option<String> = None;
    let mut files = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--samples" => {
                samples = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--csv" => {
                csv = Some(args[i + 1].clone());
                i += 2;
            }
            "--k" => {
                fixed_k = Some(args[i + 1].parse().unwrap());
                i += 2;
            }
            f => {
                files.push(f.to_string());
                i += 1;
            }
        }
    }

    // Evaluate every hand of every log.
    let mut per_file: Vec<(String, Vec<HandRes>)> = Vec::new();
    for (fi, f) in files.iter().enumerate() {
        let text = std::fs::read_to_string(f).expect("read log");
        let recs: Vec<serde_json::Value> = text
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .filter(|r: &serde_json::Value| r.get("winnings").is_some())
            .collect();
        let res: Vec<HandRes> = recs
            .par_iter()
            .enumerate()
            .filter_map(|(hi, r)| evaluate(r, samples, (fi as u64) << 32 | hi as u64))
            .collect();
        eprintln!("[AIVAT] {}: {} hands evaluated", f, res.len());
        per_file.push((f.clone(), res));
    }

    if let Some(path) = &csv {
        use std::io::Write;
        let mut w = std::io::BufWriter::new(std::fs::File::create(path).expect("create csv"));
        writeln!(w, "file,seat,action,raw,post,x,pre,flop,turn,river").unwrap();
        for (name, res) in &per_file {
            for h in res {
                writeln!(
                    w, "{},{},{},{},{:.3},{:.5},{:.3},{:.3},{:.3},{:.3}",
                    name, ["SB", "BB"][h.seat], h.action, h.raw, h.post, h.x,
                    h.streets[0], h.streets[1], h.streets[2], h.streets[3]
                ).unwrap();
            }
        }
        eprintln!("[AIVAT] wrote per-hand street terms to {}", path);
    }

    // Deal-term coefficient K per file, regressed on the other files.
    let fit_k = |exclude: usize| -> f64 {
        if let Some(k) = fixed_k {
            return k;
        }
        let (mut sxy, mut sxx) = (0.0, 0.0);
        for (j, (_, res)) in per_file.iter().enumerate() {
            if j == exclude && per_file.len() > 1 {
                continue;
            }
            for h in res {
                sxy += h.x * (h.post + h.act);
                sxx += h.x * h.x;
            }
        }
        sxy / sxx
    };

    println!(
        "{:42} {:>7} {:>16} {:>16} {:>16} {:>16} {:>7} {:>6}",
        "log", "hands", "raw", "all-in adj", "postflop terms", "full", "K", "var x"
    );
    let mut pooled_raw = Vec::new();
    let mut pooled_full = Vec::new();
    for (fi, (name, res)) in per_file.iter().enumerate() {
        let k = fit_k(fi);
        let to_mbb = |c: f64| c / 100.0 * 1000.0;
        let raw: Vec<f64> = res.iter().map(|h| to_mbb(h.raw)).collect();
        let allin: Vec<f64> = res.iter().map(|h| to_mbb(h.allin)).collect();
        let post: Vec<f64> = res.iter().map(|h| to_mbb(h.post)).collect();
        let full: Vec<f64> = res.iter().map(|h| to_mbb(h.post + h.act - k * h.x)).collect();
        let n_act: usize = res.iter().map(|h| h.n_act).sum();
        if n_act > 0 {
            let chance_only: Vec<f64> = res.iter().map(|h| to_mbb(h.post - k * h.x)).collect();
            let (mc, cc, _) = mean_ci(&chance_only);
            eprintln!(
                "[AIVAT] {}: {} decisions with values; chance-only {:+.0} ± {:.0}",
                name, n_act, mc, cc
            );
        }
        let (mr, cr, sr) = mean_ci(&raw);
        let (ma, ca, _) = mean_ci(&allin);
        let (mp, cp, _) = mean_ci(&post);
        let (mf, cf, sf) = mean_ci(&full);
        let short = name.rsplit('/').next().unwrap();
        println!(
            "{:42} {:>7} {:>+7.0} ± {:>5.0} {:>+7.0} ± {:>5.0} {:>+7.0} ± {:>5.0} {:>+7.0} ± {:>5.0} {:>7.0} {:>5.2}x",
            short, res.len(), mr, cr, ma, ca, mp, cp, mf, cf, k, (sr / sf).powi(2)
        );
        pooled_raw.extend(raw);
        pooled_full.extend(full);
    }
    let (mr, _, sr) = mean_ci(&pooled_raw);
    let (mf, _, sf) = mean_ci(&pooled_full);
    println!(
        "\nAll logs pooled: {} hands. Mean raw {:+.1}, corrected {:+.1} mbb/hand (difference should be noise).\nPer-hand std: raw {:.0}, corrected {:.0} mbb -> variance reduced {:.2}x ({:.0}% fewer hands for the same CI).",
        pooled_raw.len(), mr, mf, sr, sf, (sr / sf).powi(2), 100.0 * (1.0 - (sf / sr).powi(2))
    );
}
