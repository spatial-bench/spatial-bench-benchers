//! Harness driver for kdbush's static 2D kd-tree.
use spatial_bench_core::{
    harness::{Budget, CaseSpec},
    schema::Point,
};
use std::process::ExitCode;
pub struct Registration {
    pub run: fn(&CaseSpec, &Budget) -> Result<Point, String>,
}
pub fn harness_main(r: &[Registration]) -> ExitCode {
    if std::env::args().any(|a| a == "--list") {
        return spatial_bench_core::harness::write_registrations(
            std::io::stdout(),
            r.iter().map(|_| Vec::new()),
        )
        .map(|()| ExitCode::SUCCESS)
        .unwrap_or(ExitCode::from(4));
    }
    let s = match spatial_bench_core::harness::read_spec(std::io::stdin()) {
        Ok(s) => s,
        Err(_) => return ExitCode::from(2),
    };
    let mut o = std::io::stdout().lock();
    for c in &s.cases {
        match (r[0].run)(c, &s.budget).and_then(|p| {
            spatial_bench_core::harness::write_point(&mut o, &p).map_err(|e| format!("{e:?}"))
        }) {
            Ok(()) => (),
            Err(e) => {
                eprintln!("spatial-bench-kdbush: {e}");
                return ExitCode::from(5);
            }
        }
    }
    ExitCode::SUCCESS
}
pub use spatial_bench_measure::measure;
type Points = (Vec<[f64; 2]>, Vec<[f64; 2]>);
pub fn points(c: &CaseSpec) -> Result<Points, String> {
    let n = c.int("tree_size").ok_or("tree_size")?;
    let q = c.int("query_count").ok_or("query_count")?;
    let out = std::process::Command::new(&c.dataset_generator)
        .args([
            "--kind",
            &c.dataset,
            "--dims",
            "2",
            "--dtype",
            "f64",
            "--tree-count",
            &n.to_string(),
            "--query-count",
            &q.to_string(),
            "--seed",
            &c.random_seed.to_string(),
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("dataset".into());
    }
    let b = out.stdout;
    if b.len() < 29 || &b[..4] != b"SBDS" {
        return Err("header".into());
    }
    let n = u64::from_ne_bytes(b[13..21].try_into().map_err(|_| "header")?) as usize;
    let q = u64::from_ne_bytes(b[21..29].try_into().map_err(|_| "header")?) as usize;
    let (w, x) = b[29..].as_chunks::<8>();
    if !x.is_empty() || w.len() != (n + q) * 2 {
        return Err("body".into());
    }
    let v: Vec<f64> = w.iter().map(|x| f64::from_ne_bytes(*x)).collect();
    Ok((
        v[..n * 2].as_chunks::<2>().0.to_vec(),
        v[n * 2..].as_chunks::<2>().0.to_vec(),
    ))
}
#[macro_export]
macro_rules! bench_case {
    () => {
        $crate::Registration {
            run: |c, b| {
                let (p, q) = $crate::points(c)?;
                let n = c.int("tree_size").ok_or("tree_size")? as f64;
                let points: Vec<(f64, f64)> = p.into_iter().map(|x| (x[0], x[1])).collect();
                let tree = ::kdbush::KDBush::create(points, ::kdbush::DEFAULT_NODE_SIZE);
                let r = (100.0 / (::std::f64::consts::PI * n)).sqrt();
                $crate::measure(c, b, q.len() as u64, || {
                    q.iter().fold(0u64, |sum, x| {
                        let mut query_sum = 0_u64;
                        tree.within(x[0], x[1], r, |id| {
                            query_sum = query_sum.wrapping_add(id as u64)
                        });
                        sum.wrapping_add(query_sum)
                    })
                })
            },
        }
    };
}
