//! SHA-1 of a text as 40 hexadecimal digits: the digest the review app writes with every decision to tell which thresholds it was taken with (not a security use).

fn block(h: &mut [u32; 5], chunk: &[u8]) {
    let mut w = [0u32; 80];
    for (i, b) in chunk.chunks(4).enumerate() {
        w[i] = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }
    let [mut a, mut b, mut c, mut d, mut e] = *h;
    for (i, wi) in w.iter().enumerate() {
        let (f, k) = match i / 20 {
            0 => ((b & c) | (!b & d), 0x5A82_7999),
            1 => (b ^ c ^ d, 0x6ED9_EBA1),
            2 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
            _ => (b ^ c ^ d, 0xCA62_C1D6u32),
        };
        let t = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(*wi);
        (e, d, c, b, a) = (d, c, b.rotate_left(30), a, t);
    }
    for (x, y) in h.iter_mut().zip([a, b, c, d, e]) {
        *x = x.wrapping_add(y);
    }
}

/// `hashlib.sha1(text.encode()).hexdigest()`.
pub fn sha1_hex(text: &str) -> String {
    let mut h: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let mut data = text.as_bytes().to_vec();
    let bits = (data.len() as u64) * 8;
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bits.to_be_bytes());
    for chunk in data.chunks(64) {
        block(&mut h, chunk);
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}
