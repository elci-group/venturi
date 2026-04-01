use venturi::runtime::Runtime;
use std::time::Instant;
use std::path::Path;

fn main() {
    use venturi::runtime::Runtime;
    use std::time::Instant;
    use std::path::Path;

    // Use current directory for pits
    let pit_store_path = Path::new("pits.json");
    
    let mut runtime = Runtime::new(pit_store_path).unwrap();
    let vt_path = Path::new("benches/arithmetic.vt");
    
    // Initial parse & setup
    runtime.load_vt_file(vt_path).unwrap();
    
    let start = Instant::now();
    for _ in 0..10_000 {
        let mut loop_rt = Runtime::new(pit_store_path).unwrap();
        loop_rt.load_vt_file(vt_path).unwrap();
    }
    let duration = start.elapsed();

    println!("Venturi parsing and graph generation throughput (10,000 iterations): {:?}", duration);
}
