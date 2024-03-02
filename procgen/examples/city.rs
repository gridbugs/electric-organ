use procgen::city::Map4;
use rand::{rngs::StdRng, Rng, SeedableRng};

fn main() {
    let mut rng1 = StdRng::from_entropy();
    let rng_seed = rng1.gen::<u64>();
    //    let rng_seed = 11251360553627691406;
    println!("Rng seed: {}", rng_seed);
    let mut rng = StdRng::seed_from_u64(rng_seed);
    let map = Map4::generate(&mut rng);
    map.print();
}
