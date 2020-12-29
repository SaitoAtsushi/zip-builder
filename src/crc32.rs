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

pub trait CRC32 {
  fn crc32(self) -> u32;
}

impl<'a, T: Iterator<Item = &'a u8> + 'a> CRC32 for T {
  fn crc32(self) -> u32 {
    !self.fold(0xFFFFFFFFu32, |crc, &byte| {
      CRC_TABLE[(crc as u8 ^ byte) as usize] ^ (crc >> 8)
    })
  }
}

#[cfg(test)]
mod test {
  use super::CRC32;

  fn crc_test(s: &str, crc: u32) {
    assert_eq!(s.as_bytes().iter().crc32(), crc);
  }

  #[test]
  fn it_works() {
    crc_test("abcd", 0xed82cd11u32);
    crc_test("123456789", 0xcbf43926u32);
  }
}
