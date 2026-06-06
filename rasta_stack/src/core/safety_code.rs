// Minimal MD4 implementation for RaSTA Safety Code (no_std)
// Based on RFC 1320

pub struct Md4 {
    state: [u32; 4],
    buffer: [u8; 64],
    count: u64,
}

impl Default for Md4 {
    fn default() -> Self {
        Self::new()
    }
}

impl Md4 {
    pub fn new() -> Self {
        Md4 {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476],
            buffer: [0; 64],
            count: 0,
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        for &byte in data {
            let index = (self.count >> 3) as usize & 0x3f;
            if let Some(buf_byte) = self.buffer.get_mut(index) {
                *buf_byte = byte;
            }
            self.count += 8;
            if index == 63 {
                self.transform_internal();
            }
        }
    }

    pub fn finalize(mut self) -> [u8; 16] {
        let count_bytes = self.count.to_le_bytes();
        let index = (self.count >> 3) as usize & 0x3f;
        let pad_len = if index < 56 { 56 - index } else { 120 - index };

        let mut padding = [0u8; 120];
        if let Some(p) = padding.get_mut(0) {
            *p = 0x80;
        }

        if let Some(p_slice) = padding.get(..pad_len) {
            self.update(p_slice);
        }
        self.update(&count_bytes);

        let mut out = [0u8; 16];
        for i in 0..4 {
            if let (Some(dst), Some(s)) = (out.get_mut(i * 4..(i + 1) * 4), self.state.get(i)) {
                dst.copy_from_slice(&s.to_le_bytes());
            }
        }
        out
    }

    fn transform_internal(&mut self) {
        let mut x = [0u32; 16];
        for (i, xi) in x.iter_mut().enumerate() {
            if let Some(Ok(bytes)) = self
                .buffer
                .get(i * 4..i * 4 + 4)
                .map(|chunk| chunk.try_into())
            {
                *xi = u32::from_le_bytes(bytes);
            }
        }

        let mut a = *self.state.first().unwrap_or(&0);
        let mut b = *self.state.get(1).unwrap_or(&0);
        let mut c = *self.state.get(2).unwrap_or(&0);
        let mut d = *self.state.get(3).unwrap_or(&0);

        #[inline]
        fn f(x: u32, y: u32, z: u32) -> u32 {
            (x & y) | (!x & z)
        }
        #[inline]
        fn g(x: u32, y: u32, z: u32) -> u32 {
            (x & y) | (x & z) | (y & z)
        }
        #[inline]
        fn h(x: u32, y: u32, z: u32) -> u32 {
            x ^ y ^ z
        }
        #[inline]
        fn rot(x: u32, s: u32) -> u32 {
            x.rotate_left(s)
        }

        // Round 1
        for i in 0..4 {
            a = rot(
                a.wrapping_add(f(b, c, d))
                    .wrapping_add(*x.get(i * 4).unwrap_or(&0)),
                3,
            );
            d = rot(
                d.wrapping_add(f(a, b, c))
                    .wrapping_add(*x.get(i * 4 + 1).unwrap_or(&0)),
                7,
            );
            c = rot(
                c.wrapping_add(f(d, a, b))
                    .wrapping_add(*x.get(i * 4 + 2).unwrap_or(&0)),
                11,
            );
            b = rot(
                b.wrapping_add(f(c, d, a))
                    .wrapping_add(*x.get(i * 4 + 3).unwrap_or(&0)),
                19,
            );
        }

        // Round 2
        for i in 0..4 {
            a = rot(
                a.wrapping_add(g(b, c, d))
                    .wrapping_add(*x.get(i).unwrap_or(&0))
                    .wrapping_add(0x5a827999),
                3,
            );
            d = rot(
                d.wrapping_add(g(a, b, c))
                    .wrapping_add(*x.get(i + 4).unwrap_or(&0))
                    .wrapping_add(0x5a827999),
                5,
            );
            c = rot(
                c.wrapping_add(g(d, a, b))
                    .wrapping_add(*x.get(i + 8).unwrap_or(&0))
                    .wrapping_add(0x5a827999),
                9,
            );
            b = rot(
                b.wrapping_add(g(c, d, a))
                    .wrapping_add(*x.get(i + 12).unwrap_or(&0))
                    .wrapping_add(0x5a827999),
                13,
            );
        }

        // Round 3
        let indices = [0, 8, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15];
        for i in 0..4 {
            a = rot(
                a.wrapping_add(h(b, c, d))
                    .wrapping_add(
                        *x.get(*indices.get(i * 4).unwrap_or(&0) as usize)
                            .unwrap_or(&0),
                    )
                    .wrapping_add(0x6ed9eba1),
                3,
            );
            d = rot(
                d.wrapping_add(h(a, b, c))
                    .wrapping_add(
                        *x.get(*indices.get(i * 4 + 1).unwrap_or(&0) as usize)
                            .unwrap_or(&0),
                    )
                    .wrapping_add(0x6ed9eba1),
                9,
            );
            c = rot(
                c.wrapping_add(h(d, a, b))
                    .wrapping_add(
                        *x.get(*indices.get(i * 4 + 2).unwrap_or(&0) as usize)
                            .unwrap_or(&0),
                    )
                    .wrapping_add(0x6ed9eba1),
                11,
            );
            b = rot(
                b.wrapping_add(h(c, d, a))
                    .wrapping_add(
                        *x.get(*indices.get(i * 4 + 3).unwrap_or(&0) as usize)
                            .unwrap_or(&0),
                    )
                    .wrapping_add(0x6ed9eba1),
                15,
            );
        }

        if let Some(s) = self.state.get_mut(0) {
            *s = s.wrapping_add(a);
        }
        if let Some(s) = self.state.get_mut(1) {
            *s = s.wrapping_add(b);
        }
        if let Some(s) = self.state.get_mut(2) {
            *s = s.wrapping_add(c);
        }
        if let Some(s) = self.state.get_mut(3) {
            *s = s.wrapping_add(d);
        }
    }
}
