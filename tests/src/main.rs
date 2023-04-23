mod capsule;
mod testtool;

fn main() {
    match std::env::args().nth(1).filter(|arg| !arg.is_empty()) {
        Some(arg) => match arg.as_str() {
            "capsule" => capsule::run(),
            "testtool" => testtool::run(),
            _ => panic!("Unknown test case: {}", arg),
        },
        None => {
            // Run all tests
            capsule::run();
            testtool::run();
        }
    }
}
