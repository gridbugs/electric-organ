use procgen::city::{Map, TentacleSpec};
use rand::{rngs::StdRng, Rng, SeedableRng};

fn main() {
    let mut rng1 = StdRng::from_entropy();
    let rng_seed = rng1.gen::<u64>();
    //    let rng_seed = 11251360553627691406;
    println!("Rng seed: {}", rng_seed);
    let mut rng = StdRng::seed_from_u64(rng_seed);
    let tentacle_spec = TentacleSpec {
        num_tentacles: 3,
        segment_length: 1.5,
        distance_from_centre: 35.0,
        spread: 0.3,
    };
    let map = Map::generate(&tentacle_spec, &mut rng);
    map.print();
}
