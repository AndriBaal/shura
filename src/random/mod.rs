pub use rand;
use rand::{distributions::uniform, thread_rng, Rng};
pub fn gen_range<T: uniform::SampleUniform, R: uniform::SampleRange<T>>(range: R) -> T {
    thread_rng().gen_range(range)
}
pub fn gen_bool(p: f64) -> bool {
    thread_rng().gen_bool(p)
}
