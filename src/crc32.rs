use std::default::Default;

pub struct CRC32(u32);

impl Default for CRC32 {
    fn default() -> Self {
        CRC32(0xFFFFFFFFu32)
    }
}

impl CRC32 {
    pub fn finish(&self) -> u32 {
        !self.0
    }

    pub fn write(&mut self, bytes: &[u8]) {
        self.0 = bytes.iter().fold(self.0, |crc, &byte| {
            CRC_TABLE[(crc as u8 ^ byte) as usize] ^ (crc >> 8)
        })
    }
}

const fn make_crc_table() -> [u32; 256] {
    let mut table: [u32; 256] = [0; 256];
    let mut n = 0;
    while n != 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k != 8 {
            if c & 1 == 1 {
                c = 0xedb88320u32 ^ (c >> 1);
            } else {
                c = c >> 1;
            }
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
}

const CRC_TABLE: [u32; 256] = make_crc_table();

#[cfg(test)]
mod test {
    use super::CRC32;

    fn crc_test(s: &str, crc: u32) {
        let mut hasher = CRC32::default();
        hasher.write(s.as_bytes());
        assert_eq!(hasher.finish(), crc);
    }

    #[test]
    fn it_works() {
        crc_test("abcd", 0xed82cd11u32);
        crc_test("123456789", 0xcbf43926u32);
    }
}
