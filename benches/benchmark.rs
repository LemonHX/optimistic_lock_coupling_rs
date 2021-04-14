use criterion::criterion_main;

mod benchmarks;
criterion_main! {
    benchmarks::compare_heavy_read::lock_heavy_read,
    benchmarks::compare_heavy_write::lock_heavy_write,
    benchmarks::compare_read_only::lock_read_only,
    benchmarks::compare_write_only::lock_write_only,
}
