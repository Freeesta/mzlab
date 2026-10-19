#![no_main]

#[path = "../ingressi_lettori.rs"]
#[allow(dead_code)]
mod ingressi_lettori;

libfuzzer_sys::fuzz_target!(|data: &[u8]| ingressi_lettori::lettori_andi(data));
