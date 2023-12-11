pub use rand::*;

pub fn gen_range<
    T: distributions::uniform::SampleUniform,
    R: distributions::uniform::SampleRange<T>,
>(
    range: R,
) -> T {
    thread_rng().gen_range(range)
}
pub fn gen_bool(p: f64) -> bool {
    thread_rng().gen_bool(p)
}
