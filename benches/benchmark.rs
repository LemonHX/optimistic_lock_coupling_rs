use criterion::criterion_main;

mod benchmarks;
criterion_main! {
    benchmarks::compare_heavy_read::lock_heavy_read
}
