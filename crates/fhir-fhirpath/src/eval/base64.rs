const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BASE64URL_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64_encode_with(input: &[u8], alphabet: &[u8]) -> String {
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 {
            chunk[1] as usize
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            chunk[2] as usize
        } else {
            0
        };
        out.push(alphabet[(b0 >> 2) & 0x3f] as char);
        out.push(alphabet[((b0 << 4) | (b1 >> 4)) & 0x3f] as char);
        if chunk.len() > 1 {
            out.push(alphabet[((b1 << 2) | (b2 >> 6)) & 0x3f] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(alphabet[b2 & 0x3f] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub(super) fn base64_encode(input: &[u8]) -> String {
    base64_encode_with(input, BASE64_CHARS)
}

pub(super) fn base64_encode_url(input: &[u8]) -> String {
    base64_encode_with(input, BASE64URL_CHARS).replace('=', "")
}

fn base64_decode_with(input: &[u8], alphabet: &[u8]) -> Option<Vec<u8>> {
    let mut table = [-1i8; 256];
    for (i, &c) in alphabet.iter().enumerate() {
        table[c as usize] = i as i8;
    }
    let cleaned: Vec<u8> = input
        .iter()
        .copied()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();
    let input = cleaned.as_slice();
    let data = match input.iter().position(|&c| c == b'=') {
        Some(pad) if input[pad..].iter().all(|&c| c == b'=') => &input[..pad],
        Some(_) => return None,
        None => input,
    };
    // A single trailing sextet cannot encode a whole byte.
    if data.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::new();
    for chunk in data.chunks(4) {
        let mut sextets = [0u8; 4];
        for (slot, &c) in sextets.iter_mut().zip(chunk) {
            let v = table[c as usize];
            if v < 0 {
                return None;
            }
            *slot = v as u8;
        }
        out.push((sextets[0] << 2) | (sextets[1] >> 4));
        if chunk.len() > 2 {
            out.push((sextets[1] << 4) | (sextets[2] >> 2));
        }
        if chunk.len() > 3 {
            out.push((sextets[2] << 6) | sextets[3]);
        }
    }
    Some(out)
}

pub(super) fn base64_decode(input: &[u8]) -> Option<Vec<u8>> {
    base64_decode_with(input, BASE64_CHARS)
}

pub(super) fn base64_decode_url(input: &[u8]) -> Option<Vec<u8>> {
    base64_decode_with(input, BASE64URL_CHARS)
}
