// Testing structs with combination of primitives and &str.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

#[derive(Serialize)]
struct Timestamp(u64);

fn main() {
    let s = Timestamp(100);
    let mut buf = [0; 128];

    let (store, _) = s.encode(&mut buf);
    assert_eq!(format!("{}", s.0), format!("{}", store))
}
