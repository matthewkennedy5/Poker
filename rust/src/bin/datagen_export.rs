//! Converts `datagen` records (bincode) into flat little-endian arrays for the
//! value-net trainer (`nets/train_turn.py`), with hands in the canonical 1326
//! order (pairs a < b of solver card ids, lexicographic; `Hands::new(&[])`).
//!
//! Output files in `<out>`:
//!   boards.u8    N x 4      solver card ids
//!   scalars.f32  N x 4      pot, stack, exploitability (pot units), source
//!   index.u64    N          spot index (unique across shards)
//!   ranges.f16   N x 2 x 1326  [OOP, IP] weights (max 1, 0 on board cards)
//!   values.f16   N x 2 x 1326  value in pot units, NaN outside the range
//!   feats.f16    N x 1326 x F  `hand_features` (strength vs a random hand)
//!   meta.json
//!
//! Usage: datagen-export --out <dir> [--limit N] <turn_0.bin> [<turn_1.bin> ...]
use half::f16;
use optimus::spots::{hand_features, Sample, N_HAND_FEATURES};
use optimus::Hands;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

const N_HANDS: usize = 1326;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = None;
    let mut limit = u64::MAX;
    let mut inputs = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                out = Some(args[i + 1].clone());
                i += 1;
            }
            "--limit" => {
                limit = args[i + 1].parse().unwrap();
                i += 1;
            }
            a => inputs.push(a.to_string()),
        }
        i += 1;
    }
    let out = out.expect("--out <dir>");
    std::fs::create_dir_all(&out).unwrap();
    let open = |name: &str| BufWriter::new(File::create(format!("{}/{}", out, name)).unwrap());
    let (mut boards, mut scalars, mut index) = (open("boards.u8"), open("scalars.f32"), open("index.u64"));
    let (mut ranges, mut values, mut feats) = (open("ranges.f16"), open("values.f16"), open("feats.f16"));
    let canon = Hands::new(&[]);
    assert_eq!(canon.len(), N_HANDS);
    let f16_bytes = |xs: &[f16]| xs.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();

    let mut n = 0u64;
    let mut done = false;
    for path in &inputs {
        let mut r = BufReader::new(File::open(path).unwrap());
        while !done {
            // A shard still being written may end in a partial record: stop there.
            let mut chunk = Vec::new();
            while chunk.len() < 4096 && n + (chunk.len() as u64) < limit {
                match bincode::deserialize_from::<_, Sample>(&mut r) {
                    Ok(s) => chunk.push(s),
                    Err(_) => break,
                }
            }
            if chunk.is_empty() {
                break;
            }
            done = n + chunk.len() as u64 >= limit;
            let encoded: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)> = chunk
                .par_iter()
                .map(|s| {
                    assert_eq!(s.spot.board.len(), 4, "turn spots only");
                    let hands = Hands::new(&s.spot.board);
                    let hf = hand_features(&s.spot.board, &hands);
                    let mut w = vec![f16::ZERO; 2 * N_HANDS];
                    let mut v = vec![f16::NAN; 2 * N_HANDS];
                    let mut f = vec![f16::ZERO; N_HANDS * N_HAND_FEATURES];
                    for (k, &(a, b)) in hands.cards.iter().enumerate() {
                        let c = canon.index(a, b).unwrap();
                        for p in 0..2 {
                            w[p * N_HANDS + c] = f16::from_f32(s.spot.weights[p][k]);
                            if s.spot.weights[p][k] > 0.0 {
                                v[p * N_HANDS + c] = f16::from_f32(s.values[p][k]);
                            }
                        }
                        for j in 0..N_HAND_FEATURES {
                            f[c * N_HAND_FEATURES + j] = f16::from_f32(hf[k][j]);
                        }
                    }
                    (f16_bytes(&w), f16_bytes(&v), f16_bytes(&f))
                })
                .collect();
            for (s, (w, v, f)) in chunk.iter().zip(encoded) {
                boards.write_all(&s.spot.board).unwrap();
                for x in [s.spot.pot as f32, s.spot.stack as f32, s.exploitability, s.spot.source as f32] {
                    scalars.write_all(&x.to_le_bytes()).unwrap();
                }
                index.write_all(&s.index.to_le_bytes()).unwrap();
                ranges.write_all(&w).unwrap();
                values.write_all(&v).unwrap();
                feats.write_all(&f).unwrap();
            }
            n += chunk.len() as u64;
        }
        eprintln!("[EXPORT] {}: {} records so far", path, n);
    }
    for w in [&mut boards, &mut scalars, &mut index, &mut ranges, &mut values, &mut feats] {
        w.flush().unwrap();
    }
    let meta = serde_json::json!({
        "n": n,
        "n_hands": N_HANDS,
        "n_features": N_HAND_FEATURES,
        "board_cards": 4,
        "hand_order": "pairs a < b of solver card ids (4 * rank + suit), lexicographic",
        "inputs": inputs,
    });
    std::fs::write(format!("{}/meta.json", out), serde_json::to_string_pretty(&meta).unwrap()).unwrap();
    println!("[EXPORT] {} records -> {}", n, out);
}
