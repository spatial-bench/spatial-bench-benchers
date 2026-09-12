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
            Err(_) => return ExitCode::from(5),
        }
    }
    ExitCode::SUCCESS
}
pub use spatial_bench_measure::measure;
type Points = (Vec<[f32; 3]>, Vec<[f32; 3]>);
pub fn points(c: &CaseSpec) -> Result<Points, String> {
    let n = c.int("tree_size").ok_or("tree_size")?;
    let q = c.int("query_count").ok_or("query_count")?;
    let out = std::process::Command::new(&c.dataset_generator)
        .args([
            "--kind",
            &c.dataset,
            "--dims",
            "3",
            "--dtype",
            "f32",
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
    if b.len() < 29 {
        return Err("header".into());
    }
    let n = u64::from_ne_bytes(b[13..21].try_into().map_err(|_| "header")?) as usize;
    let q = u64::from_ne_bytes(b[21..29].try_into().map_err(|_| "header")?) as usize;
    let (w, x) = b[29..].as_chunks::<4>();
    if !x.is_empty() || w.len() != (n + q) * 3 {
        return Err("body".into());
    }
    let v: Vec<f32> = w.iter().map(|x| f32::from_ne_bytes(*x)).collect();
    Ok((
        v[..n * 3].as_chunks::<3>().0.to_vec(),
        v[n * 3..].as_chunks::<3>().0.to_vec(),
    ))
}
#[macro_export]
macro_rules! bench_case {
    () => {
        $crate::Registration {
            run: |c, b| {
                let (p, q) = $crate::points(c)?;
                let t = ::neighbourhood::KdTree::new(p);
                let r = ((3.0 * 100.0)
                    / (4.0
                        * ::std::f32::consts::PI
                        * c.int("tree_size").ok_or("tree_size")? as f32))
                    .cbrt();
                $crate::measure(c, b, q.len() as u64, || {
                    q.iter().fold(0u64, |s, x| {
                        s.wrapping_add(
                            t.neighbourhood(x, r)
                                .iter()
                                .fold(0u64, |a, point| a.wrapping_add(point[0].to_bits() as u64)),
                        )
                    })
                })
            },
        }
    };
}
