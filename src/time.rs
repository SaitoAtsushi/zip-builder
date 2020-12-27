use std::default::Default;
use std::iter::Iterator;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

struct EpochTime(u64);

impl EpochTime {
    fn new(st: &SystemTime) -> EpochTime {
        EpochTime(match st.duration_since(UNIX_EPOCH) {
            Ok(n) => n.as_secs(),
            Err(_) => 0,
        })
    }
}

impl Default for EpochTime {
    fn default() -> Self {
        EpochTime::new(&SystemTime::now())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateTime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

impl DateTime {
    pub fn now() -> DateTime {
        DateTime::from(&EpochTime::default())
    }

    pub fn dos_time(&self) -> u32 {
        if self.year >= 1980 {
            ((self.year - 1980) as u32) << 25
                | (self.month as u32).wrapping_shl(21)
                | ((self.day as u32) << 16)
                | ((self.hour as u32) << 11)
                | ((self.minute as u32) << 5)
                | ((self.second as u32) >> 1)
        } else {
            0
        }
    }
}

impl Default for DateTime {
    fn default() -> Self {
        DateTime::now()
    }
}

fn year_from_days(days: u64) -> (u16, u16) {
    if days > 10957 {
        let mut year = 0;
        let mut days = days - 10957; // 10957 is days from epoch to millennium
        for (days_in_a_year, years) in [
            (400 * 365 + 97, 400),
            (100 * 365 + 24, 100),
            (4 * 365 + 1, 4),
            (365, 1),
        ]
        .iter()
        {
            year += days / days_in_a_year * years;
            days %= days_in_a_year;
        }
        ((year + 2000) as u16, days as u16)
    } else {
        let mut year = 0;
        let mut days = days + 3653;
        for (days_in_a_year, years) in [(4 * 365 + 1, 4), (365, 1)].iter() {
            year += days / days_in_a_year * years;
            days %= days_in_a_year;
        }
        ((year + 1960) as u16, (days + 1) as u16)
    }
}

const fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0) & ((year % 100 != 0) | (year % 400 == 0))
}

const DAYS_IN_YEAR: [u16; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const DAYS_IN_YEAR_OF_LEAP_YEAR: [u16; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

fn month_from_days(mut days: u16, is_leap: bool) -> (u8, u8) {
    (if is_leap {
        DAYS_IN_YEAR
    } else {
        DAYS_IN_YEAR_OF_LEAP_YEAR
    })
    .iter()
    .enumerate()
    .find_map(|(num, cum)| {
        if *cum > days {
            Some(((num + 1) as u8, days as u8))
        } else {
            days -= cum;
            None
        }
    })
    .unwrap()
}

impl From<&EpochTime> for DateTime {
    fn from(et: &EpochTime) -> Self {
        let second = (et.0 % 60) as u8;
        let rest = et.0 / 60;
        let minute = (rest % 60) as u8;
        let rest = rest / 60;
        let hour = (rest % 24) as u8;
        let rest = rest / 24;
        let (year, days) = year_from_days(rest);
        let (month, days) = month_from_days(days, is_leap_year(year));
        DateTime {
            year: year,
            month: month,
            day: days,
            hour: hour,
            minute: minute,
            second: second,
        }
    }
}

#[cfg(test)]
mod test {
    use super::{DateTime, EpochTime};
    use std::convert::From;

    fn time_test(dt: &DateTime, et: u64, dos_time: u32) {
        assert_eq!(*dt, DateTime::from(&EpochTime(et)));
        assert_eq!(dt.dos_time(), dos_time);
    }

    #[test]
    fn it_works() {
        time_test(
            &DateTime {
                year: 1970,
                month: 10,
                day: 14,
                hour: 21,
                minute: 2,
                second: 30,
            },
            24786150,
            0u32,
        );
        time_test(
            &DateTime {
                year: 1980,
                month: 1,
                day: 7,
                hour: 18,
                minute: 5,
                second: 10,
            },
            316116310,
            2592933,
        );
        time_test(
            &DateTime {
                year: 1998,
                month: 2,
                day: 1,
                hour: 19,
                minute: 5,
                second: 2,
            },
            886273502,
            608278689,
        );
        time_test(
            &DateTime {
                year: 2000,
                month: 3,
                day: 12,
                hour: 5,
                minute: 4,
                second: 1,
            },
            952837441,
            678176896,
        );
        time_test(
            &DateTime {
                year: 2020,
                month: 12,
                day: 25,
                hour: 14,
                minute: 5,
                second: 23,
            },
            1608905123,
            1369010347,
        );
    }
}
