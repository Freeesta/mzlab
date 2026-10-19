#![no_main]

#[path = "../ingressi.rs"]
#[allow(dead_code)]
mod ingressi;

libfuzzer_sys::fuzz_target!(|data: &[u8]| ingressi::xmltree_methodinfo(data));
