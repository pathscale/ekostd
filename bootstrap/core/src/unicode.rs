//! Unicode tables for `char::escape_debug`, copied from the Rust standard library (`core/src/unicode/unicode_data.rs`,
//! Unicode 17.0.0): `Grapheme_Extend`, `Cf`, `Cn` in planes 0 to 3 and `Default_Ignorable_Code_Point`, with the
//! `skip_search` they share. The compile-time table checks and the `assume` hints are dropped; the tables are untouched.

#[repr(transparent)]
struct ShortOffsetRunHeader(u32);

impl ShortOffsetRunHeader {
    const fn new(start_index: usize, prefix_sum: u32) -> Self {
        assert!(start_index < (1 << 11));
        assert!(prefix_sum < (1 << 21));

        Self((start_index as u32) << 21 | prefix_sum)
    }

    #[inline]
    const fn start_index(&self) -> usize {
        (self.0 >> 21) as usize
    }

    #[inline]
    const fn prefix_sum(&self) -> u32 {
        self.0 & ((1 << 21) - 1)
    }
}

/// # Safety
///
/// - The last element of `short_offset_runs` must be greater than `std::char::MAX`.
/// - The start indices of all elements in `short_offset_runs` must be less than `OFFSETS`.
#[inline(always)]
unsafe fn skip_search<const SOR: usize, const OFFSETS: usize>(
    needle: char,
    short_offset_runs: &[ShortOffsetRunHeader; SOR],
    offsets: &[u8; OFFSETS],
) -> bool {
    let needle = needle as u32;

    let last_idx =
        match short_offset_runs.binary_search_by_key(&(needle << 11), |header| header.0 << 11) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        };

    let mut offset_idx = short_offset_runs[last_idx].start_index();
    let length = if let Some(next) = short_offset_runs.get(last_idx + 1) {
        (*next).start_index() - offset_idx
    } else {
        offsets.len() - offset_idx
    };

    let prev =
        last_idx.checked_sub(1).map(|prev| short_offset_runs[prev].prefix_sum()).unwrap_or(0);

    let total = needle - prev;
    let mut prefix_sum = 0;
    for _ in 0..(length - 1) {
        let offset = offsets[offset_idx];
        prefix_sum += offset as u32;
        if prefix_sum > total {
            break;
        }
        offset_idx += 1;
    }
    offset_idx % 2 == 1
}

pub const UNICODE_VERSION: (u8, u8, u8) = (17, 0, 0);

pub mod cf {
    use super::ShortOffsetRunHeader;

    static SHORT_OFFSET_RUNS: [ShortOffsetRunHeader; 11] = [
        ShortOffsetRunHeader::new(0, 1536), ShortOffsetRunHeader::new(3, 2192),
        ShortOffsetRunHeader::new(11, 6158), ShortOffsetRunHeader::new(15, 8203),
        ShortOffsetRunHeader::new(17, 65279), ShortOffsetRunHeader::new(25, 69821),
        ShortOffsetRunHeader::new(29, 78896), ShortOffsetRunHeader::new(33, 113824),
        ShortOffsetRunHeader::new(35, 119155), ShortOffsetRunHeader::new(37, 917505),
        ShortOffsetRunHeader::new(39, 2031744),
    ];
    static OFFSETS: [u8; 43] = [
        173, 1, 0, 6, 22, 1, 192, 1, 49, 1, 0, 2, 80, 1, 0, 1, 0, 5, 26, 5, 49, 5, 1, 10, 0, 1,
        249, 3, 0, 1, 15, 1, 0, 16, 0, 4, 0, 8, 0, 1, 30, 96, 0,
    ];
    #[inline]
    pub fn lookup(c: char) -> bool {
        debug_assert!(!c.is_ascii());
        (c as u32) >= 0xad && lookup_slow(c)
    }

    #[inline(never)]
    fn lookup_slow(c: char) -> bool {
        unsafe { super::skip_search(c, &SHORT_OFFSET_RUNS, &OFFSETS) }
    }
}

pub mod cn_planes_0_3 {
    use super::ShortOffsetRunHeader;

    static SHORT_OFFSET_RUNS: [ShortOffsetRunHeader; 54] = [
        ShortOffsetRunHeader::new(0, 888), ShortOffsetRunHeader::new(1, 1328),
        ShortOffsetRunHeader::new(11, 1806), ShortOffsetRunHeader::new(25, 4681),
        ShortOffsetRunHeader::new(325, 5789), ShortOffsetRunHeader::new(365, 7958),
        ShortOffsetRunHeader::new(445, 9258), ShortOffsetRunHeader::new(491, 11124),
        ShortOffsetRunHeader::new(495, 11508), ShortOffsetRunHeader::new(497, 42125),
        ShortOffsetRunHeader::new(549, 42540), ShortOffsetRunHeader::new(553, 55204),
        ShortOffsetRunHeader::new(605, 64110), ShortOffsetRunHeader::new(611, 64976),
        ShortOffsetRunHeader::new(629, 67383), ShortOffsetRunHeader::new(735, 74650),
        ShortOffsetRunHeader::new(1067, 77712), ShortOffsetRunHeader::new(1074, 78934),
        ShortOffsetRunHeader::new(1077, 82939), ShortOffsetRunHeader::new(1079, 83527),
        ShortOffsetRunHeader::new(1081, 90368), ShortOffsetRunHeader::new(1082, 92160),
        ShortOffsetRunHeader::new(1084, 92729), ShortOffsetRunHeader::new(1085, 93504),
        ShortOffsetRunHeader::new(1108, 101590), ShortOffsetRunHeader::new(1127, 110576),
        ShortOffsetRunHeader::new(1132, 110883), ShortOffsetRunHeader::new(1139, 111356),
        ShortOffsetRunHeader::new(1149, 113664), ShortOffsetRunHeader::new(1150, 117760),
        ShortOffsetRunHeader::new(1160, 118452), ShortOffsetRunHeader::new(1163, 120486),
        ShortOffsetRunHeader::new(1227, 120780), ShortOffsetRunHeader::new(1229, 121484),
        ShortOffsetRunHeader::new(1231, 122624), ShortOffsetRunHeader::new(1236, 123536),
        ShortOffsetRunHeader::new(1262, 124112), ShortOffsetRunHeader::new(1268, 126065),
        ShortOffsetRunHeader::new(1298, 126976), ShortOffsetRunHeader::new(1370, 128729),
        ShortOffsetRunHeader::new(1395, 129624), ShortOffsetRunHeader::new(1423, 131072),
        ShortOffsetRunHeader::new(1444, 173792), ShortOffsetRunHeader::new(1445, 178206),
        ShortOffsetRunHeader::new(1447, 183982), ShortOffsetRunHeader::new(1449, 191457),
        ShortOffsetRunHeader::new(1451, 192094), ShortOffsetRunHeader::new(1453, 194560),
        ShortOffsetRunHeader::new(1454, 195102), ShortOffsetRunHeader::new(1455, 196608),
        ShortOffsetRunHeader::new(1456, 201547), ShortOffsetRunHeader::new(1457, 210042),
        ShortOffsetRunHeader::new(1459, 262142), ShortOffsetRunHeader::new(1460, 1376254),
    ];
    static OFFSETS: [u8; 1461] = [
        0, 2, 6, 4, 7, 1, 1, 1, 20, 1, 0, 1, 38, 2, 50, 2, 3, 1, 55, 8, 27, 4, 6, 11, 0, 1, 60, 2,
        101, 14, 59, 2, 49, 2, 15, 1, 28, 2, 1, 1, 11, 5, 34, 5, 237, 1, 8, 2, 2, 2, 22, 1, 7, 1, 1,
        3, 4, 2, 9, 2, 2, 2, 4, 8, 1, 4, 2, 1, 5, 2, 25, 2, 3, 1, 6, 4, 2, 2, 22, 1, 7, 1, 2, 1, 2,
        1, 2, 2, 1, 1, 5, 4, 2, 2, 3, 3, 1, 7, 4, 1, 1, 7, 17, 10, 3, 1, 9, 1, 3, 1, 22, 1, 7, 1, 2,
        1, 5, 2, 10, 1, 3, 1, 3, 2, 1, 15, 4, 2, 12, 7, 7, 1, 3, 1, 8, 2, 2, 2, 22, 1, 7, 1, 2, 1,
        5, 2, 9, 2, 2, 2, 3, 7, 3, 4, 2, 1, 5, 2, 18, 10, 2, 1, 6, 3, 3, 1, 4, 3, 2, 1, 1, 1, 2, 3,
        2, 3, 3, 3, 12, 4, 5, 3, 3, 1, 4, 2, 1, 6, 1, 14, 21, 5, 13, 1, 3, 1, 23, 1, 16, 2, 9, 1, 3,
        1, 4, 7, 2, 1, 3, 1, 2, 2, 4, 2, 10, 7, 22, 1, 3, 1, 23, 1, 10, 1, 5, 2, 9, 1, 3, 1, 4, 7,
        2, 5, 3, 1, 4, 2, 10, 1, 3, 12, 13, 1, 3, 1, 51, 1, 3, 1, 6, 4, 16, 2, 26, 1, 3, 1, 18, 3,
        24, 1, 9, 1, 1, 2, 7, 3, 1, 4, 6, 1, 1, 1, 8, 6, 10, 2, 3, 12, 58, 4, 29, 37, 2, 1, 1, 1, 5,
        1, 24, 1, 1, 1, 23, 2, 5, 1, 1, 1, 7, 1, 10, 2, 4, 32, 72, 1, 36, 4, 39, 1, 36, 1, 15, 1,
        13, 37, 198, 1, 1, 5, 1, 2, 0, 1, 4, 2, 7, 1, 1, 1, 4, 2, 41, 1, 4, 2, 33, 1, 4, 2, 7, 1, 1,
        1, 4, 2, 15, 1, 57, 1, 4, 2, 67, 2, 32, 3, 26, 6, 86, 2, 6, 2, 0, 3, 89, 7, 22, 9, 24, 9,
        20, 12, 13, 1, 3, 1, 2, 12, 94, 2, 10, 6, 10, 6, 26, 6, 89, 7, 43, 5, 70, 10, 31, 1, 12, 4,
        12, 4, 1, 3, 42, 2, 5, 11, 44, 4, 26, 6, 11, 3, 62, 2, 65, 1, 29, 2, 11, 6, 10, 6, 14, 2,
        46, 2, 12, 20, 77, 1, 166, 8, 60, 3, 15, 3, 62, 5, 43, 2, 11, 8, 43, 5, 0, 2, 6, 2, 38, 2,
        6, 2, 8, 1, 1, 1, 1, 1, 1, 1, 31, 2, 53, 1, 15, 1, 14, 2, 6, 1, 19, 2, 3, 1, 9, 1, 101, 1,
        12, 2, 27, 1, 13, 3, 34, 14, 33, 15, 140, 4, 0, 22, 11, 21, 0, 2, 0, 5, 45, 1, 1, 5, 1, 2,
        56, 7, 2, 14, 24, 9, 7, 1, 7, 1, 7, 1, 7, 1, 7, 1, 7, 1, 7, 1, 7, 1, 126, 34, 26, 1, 89, 12,
        214, 26, 80, 1, 86, 2, 103, 5, 43, 1, 94, 1, 86, 9, 48, 1, 0, 3, 55, 9, 0, 20, 184, 8, 221,
        20, 60, 3, 10, 6, 56, 8, 70, 8, 12, 6, 116, 11, 30, 3, 78, 1, 11, 4, 33, 1, 55, 9, 14, 2,
        10, 2, 103, 24, 28, 10, 6, 2, 6, 2, 6, 9, 7, 1, 7, 1, 60, 4, 126, 2, 10, 6, 0, 12, 23, 4,
        49, 4, 0, 2, 106, 38, 7, 12, 5, 5, 26, 1, 5, 1, 1, 1, 2, 1, 2, 1, 0, 32, 42, 6, 51, 1, 19,
        1, 4, 4, 5, 1, 135, 2, 1, 1, 190, 3, 6, 2, 6, 2, 6, 2, 3, 3, 7, 1, 7, 10, 5, 2, 12, 1, 26,
        1, 19, 1, 2, 1, 15, 2, 14, 34, 123, 5, 3, 4, 45, 3, 88, 1, 13, 3, 1, 47, 46, 130, 29, 3, 49,
        15, 28, 4, 36, 9, 30, 5, 43, 5, 30, 1, 37, 4, 14, 42, 158, 2, 10, 6, 36, 4, 36, 4, 40, 8,
        52, 11, 12, 1, 15, 1, 7, 1, 2, 1, 11, 1, 15, 1, 7, 1, 2, 3, 52, 12, 0, 9, 22, 10, 8, 24, 6,
        1, 42, 1, 9, 69, 6, 2, 1, 1, 44, 1, 2, 3, 1, 2, 23, 1, 72, 8, 9, 48, 19, 1, 2, 5, 33, 3, 27,
        5, 27, 38, 56, 4, 20, 2, 50, 1, 2, 5, 8, 1, 3, 1, 29, 2, 3, 4, 10, 7, 9, 7, 64, 32, 39, 4,
        12, 9, 54, 3, 29, 2, 27, 5, 26, 7, 4, 12, 7, 80, 73, 55, 51, 13, 51, 7, 46, 8, 10, 6, 38, 3,
        29, 8, 2, 208, 31, 1, 42, 1, 3, 2, 2, 16, 6, 8, 9, 33, 46, 8, 42, 22, 26, 38, 28, 20, 23, 9,
        78, 4, 36, 9, 68, 10, 1, 2, 25, 7, 10, 6, 53, 1, 18, 8, 39, 9, 96, 1, 20, 11, 18, 1, 47, 62,
        7, 1, 1, 1, 4, 1, 15, 1, 11, 6, 59, 5, 10, 6, 4, 1, 8, 2, 2, 2, 22, 1, 7, 1, 2, 1, 5, 1, 10,
        2, 2, 2, 3, 2, 1, 6, 1, 5, 7, 2, 7, 3, 5, 11, 10, 1, 1, 2, 1, 1, 38, 1, 10, 1, 1, 2, 1, 1,
        4, 1, 10, 1, 2, 8, 2, 29, 92, 1, 5, 30, 72, 8, 10, 166, 54, 2, 38, 34, 69, 11, 10, 6, 13,
        19, 58, 6, 10, 6, 20, 28, 27, 2, 15, 4, 23, 185, 60, 100, 83, 12, 8, 2, 1, 2, 8, 1, 2, 1,
        30, 1, 2, 2, 12, 9, 10, 70, 8, 2, 46, 2, 11, 27, 72, 8, 83, 13, 73, 7, 10, 86, 8, 88, 34,
        14, 10, 6, 9, 1, 45, 1, 14, 10, 29, 3, 32, 2, 22, 1, 14, 73, 7, 1, 2, 1, 44, 3, 1, 1, 2, 1,
        9, 8, 10, 6, 6, 1, 2, 1, 37, 1, 2, 1, 6, 7, 10, 6, 44, 4, 10, 246, 25, 7, 17, 1, 41, 3, 29,
        85, 1, 15, 50, 13, 0, 102, 111, 1, 5, 11, 196, 0, 99, 13, 0, 10, 0, 5, 0, 0, 58, 0, 0, 7,
        31, 1, 10, 4, 81, 1, 10, 6, 30, 2, 6, 10, 70, 10, 10, 1, 7, 1, 21, 5, 19, 0, 58, 198, 91, 5,
        25, 2, 25, 44, 75, 4, 57, 7, 17, 64, 5, 11, 7, 9, 0, 41, 32, 97, 115, 0, 4, 1, 7, 1, 2, 1,
        0, 15, 1, 29, 3, 2, 1, 14, 4, 8, 0, 0, 107, 5, 13, 3, 9, 7, 10, 2, 8, 0, 253, 3, 0, 6, 23,
        15, 17, 15, 46, 2, 23, 9, 116, 60, 246, 10, 39, 2, 194, 21, 70, 122, 20, 12, 20, 12, 87, 9,
        25, 135, 85, 1, 71, 1, 2, 2, 1, 2, 2, 2, 4, 1, 12, 1, 1, 1, 7, 1, 65, 1, 4, 2, 8, 1, 7, 1,
        28, 1, 4, 1, 5, 1, 1, 3, 7, 1, 0, 2, 0, 2, 0, 15, 5, 1, 15, 0, 31, 6, 6, 213, 7, 1, 17, 2,
        7, 1, 2, 1, 5, 5, 62, 33, 1, 112, 45, 3, 14, 2, 10, 4, 2, 0, 31, 17, 58, 5, 1, 0, 42, 214,
        43, 4, 1, 192, 31, 1, 22, 8, 2, 224, 7, 1, 4, 1, 2, 1, 15, 1, 197, 2, 16, 41, 76, 4, 10, 4,
        2, 0, 68, 76, 61, 194, 4, 1, 27, 1, 2, 1, 1, 2, 1, 1, 10, 1, 4, 1, 1, 1, 1, 6, 1, 4, 1, 1,
        1, 1, 1, 1, 3, 1, 2, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 2, 4, 1, 7, 1, 4, 1, 4,
        1, 1, 1, 10, 1, 17, 5, 3, 1, 5, 1, 17, 52, 2, 0, 44, 4, 100, 12, 15, 2, 15, 1, 15, 1, 37,
        10, 174, 56, 29, 13, 44, 4, 9, 7, 2, 14, 6, 154, 0, 3, 17, 3, 13, 3, 218, 6, 12, 4, 1, 15,
        12, 4, 56, 8, 10, 6, 40, 8, 30, 2, 12, 4, 2, 14, 9, 39, 0, 8, 14, 2, 13, 3, 11, 3, 57, 1, 1,
        4, 16, 2, 12, 4, 10, 7, 147, 1, 103, 0, 0, 32, 0, 2, 0, 2, 0, 15, 0, 0, 0, 0, 0, 5, 0, 0, 0,
    ];
    #[inline]
    pub fn lookup(c: char) -> bool {
        debug_assert!(!c.is_ascii());
        (c as u32) >= 0x378 && lookup_slow(c)
    }

    #[inline(never)]
    fn lookup_slow(c: char) -> bool {
        unsafe { super::skip_search(c, &SHORT_OFFSET_RUNS, &OFFSETS) }
    }
}

pub mod default_ignorable_code_point {
    use super::ShortOffsetRunHeader;

    static SHORT_OFFSET_RUNS: [ShortOffsetRunHeader; 12] = [
        ShortOffsetRunHeader::new(0, 847), ShortOffsetRunHeader::new(3, 1564),
        ShortOffsetRunHeader::new(5, 4447), ShortOffsetRunHeader::new(7, 6068),
        ShortOffsetRunHeader::new(9, 8203), ShortOffsetRunHeader::new(13, 12644),
        ShortOffsetRunHeader::new(19, 65024), ShortOffsetRunHeader::new(21, 113824),
        ShortOffsetRunHeader::new(29, 119155), ShortOffsetRunHeader::new(31, 917504),
        ShortOffsetRunHeader::new(33, 921600), ShortOffsetRunHeader::new(34, 2035712),
    ];
    static OFFSETS: [u8; 35] = [
        173, 1, 0, 1, 0, 1, 0, 2, 0, 2, 85, 5, 0, 5, 26, 5, 49, 16, 0, 1, 0, 16, 239, 1, 160, 1,
        79, 9, 0, 4, 0, 8, 0, 0, 0,
    ];
    #[inline]
    pub fn lookup(c: char) -> bool {
        debug_assert!(!c.is_ascii());
        (c as u32) >= 0xad && lookup_slow(c)
    }

    #[inline(never)]
    fn lookup_slow(c: char) -> bool {
        unsafe { super::skip_search(c, &SHORT_OFFSET_RUNS, &OFFSETS) }
    }
}

pub mod grapheme_extend {
    use super::ShortOffsetRunHeader;

    static SHORT_OFFSET_RUNS: [ShortOffsetRunHeader; 33] = [
        ShortOffsetRunHeader::new(0, 768), ShortOffsetRunHeader::new(1, 1155),
        ShortOffsetRunHeader::new(3, 1425), ShortOffsetRunHeader::new(5, 4957),
        ShortOffsetRunHeader::new(249, 5906), ShortOffsetRunHeader::new(251, 8204),
        ShortOffsetRunHeader::new(347, 11503), ShortOffsetRunHeader::new(351, 12330),
        ShortOffsetRunHeader::new(357, 42607), ShortOffsetRunHeader::new(361, 43010),
        ShortOffsetRunHeader::new(369, 64286), ShortOffsetRunHeader::new(435, 65024),
        ShortOffsetRunHeader::new(437, 65438), ShortOffsetRunHeader::new(441, 66045),
        ShortOffsetRunHeader::new(443, 68097), ShortOffsetRunHeader::new(449, 68900),
        ShortOffsetRunHeader::new(461, 69291), ShortOffsetRunHeader::new(465, 71727),
        ShortOffsetRunHeader::new(601, 73459), ShortOffsetRunHeader::new(669, 78912),
        ShortOffsetRunHeader::new(679, 90398), ShortOffsetRunHeader::new(683, 92912),
        ShortOffsetRunHeader::new(687, 94031), ShortOffsetRunHeader::new(691, 113821),
        ShortOffsetRunHeader::new(699, 118528), ShortOffsetRunHeader::new(701, 119141),
        ShortOffsetRunHeader::new(705, 121344), ShortOffsetRunHeader::new(717, 122880),
        ShortOffsetRunHeader::new(729, 123566), ShortOffsetRunHeader::new(743, 124140),
        ShortOffsetRunHeader::new(747, 125136), ShortOffsetRunHeader::new(759, 917536),
        ShortOffsetRunHeader::new(763, 2032112),
    ];
    static OFFSETS: [u8; 767] = [
        0, 112, 0, 7, 0, 45, 1, 1, 1, 2, 1, 2, 1, 1, 72, 11, 48, 21, 16, 1, 101, 7, 2, 6, 2, 2, 1,
        4, 35, 1, 30, 27, 91, 11, 58, 9, 9, 1, 24, 4, 1, 9, 1, 3, 1, 5, 43, 3, 59, 9, 42, 24, 1, 32,
        55, 1, 1, 1, 4, 8, 4, 1, 3, 7, 10, 2, 29, 1, 58, 1, 1, 1, 2, 4, 8, 1, 9, 1, 10, 2, 26, 1, 2,
        2, 57, 1, 4, 2, 4, 2, 2, 3, 3, 1, 30, 2, 3, 1, 11, 2, 57, 1, 4, 5, 1, 2, 4, 1, 20, 2, 22, 6,
        1, 1, 58, 1, 1, 2, 1, 4, 8, 1, 7, 3, 10, 2, 30, 1, 59, 1, 1, 1, 12, 1, 9, 1, 40, 1, 3, 1,
        55, 1, 1, 3, 5, 3, 1, 4, 7, 2, 11, 2, 29, 1, 58, 1, 2, 2, 1, 1, 3, 3, 1, 4, 7, 2, 11, 2, 28,
        2, 57, 2, 1, 1, 2, 4, 8, 1, 9, 1, 10, 2, 29, 1, 72, 1, 4, 1, 2, 3, 1, 1, 8, 1, 81, 1, 2, 7,
        12, 8, 98, 1, 2, 9, 11, 7, 73, 2, 27, 1, 1, 1, 1, 1, 55, 14, 1, 5, 1, 2, 5, 11, 1, 36, 9, 1,
        102, 4, 1, 6, 1, 2, 2, 2, 25, 2, 4, 3, 16, 4, 13, 1, 2, 2, 6, 1, 15, 1, 0, 3, 0, 4, 28, 3,
        29, 2, 30, 2, 64, 2, 1, 7, 8, 1, 2, 11, 9, 1, 45, 3, 1, 1, 117, 2, 34, 1, 118, 3, 4, 2, 9,
        1, 6, 3, 219, 2, 2, 1, 58, 1, 1, 7, 1, 1, 1, 1, 2, 8, 6, 10, 2, 1, 48, 46, 2, 12, 20, 4, 48,
        10, 4, 3, 38, 9, 12, 2, 32, 4, 2, 6, 56, 1, 1, 2, 3, 1, 1, 5, 56, 8, 2, 2, 152, 3, 1, 13, 1,
        7, 4, 1, 6, 1, 3, 2, 198, 64, 0, 1, 195, 33, 0, 3, 141, 1, 96, 32, 0, 6, 105, 2, 0, 4, 1,
        10, 32, 2, 80, 2, 0, 1, 3, 1, 4, 1, 25, 2, 5, 1, 151, 2, 26, 18, 13, 1, 38, 8, 25, 11, 1, 1,
        44, 3, 48, 1, 2, 4, 2, 2, 2, 1, 36, 1, 67, 6, 2, 2, 2, 2, 12, 1, 8, 1, 47, 1, 51, 1, 1, 3,
        2, 2, 5, 2, 1, 1, 42, 2, 8, 1, 238, 1, 2, 1, 4, 1, 0, 1, 0, 16, 16, 16, 0, 2, 0, 1, 226, 1,
        149, 5, 0, 3, 1, 2, 5, 4, 40, 3, 4, 1, 165, 2, 0, 4, 65, 5, 0, 2, 77, 6, 70, 11, 49, 4, 123,
        1, 54, 15, 41, 1, 2, 2, 10, 3, 49, 4, 2, 2, 7, 1, 61, 3, 36, 5, 1, 8, 62, 1, 12, 2, 52, 9,
        1, 1, 8, 4, 2, 1, 95, 3, 2, 4, 6, 1, 2, 1, 157, 1, 3, 8, 21, 2, 57, 2, 1, 1, 1, 1, 12, 1, 9,
        1, 14, 7, 3, 5, 67, 1, 2, 6, 1, 1, 2, 1, 1, 3, 4, 3, 1, 1, 14, 2, 85, 8, 2, 3, 1, 1, 23, 1,
        81, 1, 2, 6, 1, 1, 2, 1, 1, 2, 1, 2, 235, 1, 2, 4, 6, 2, 1, 2, 27, 2, 85, 8, 2, 1, 1, 2,
        106, 1, 1, 1, 2, 8, 101, 1, 1, 1, 2, 4, 1, 5, 0, 9, 1, 2, 245, 1, 10, 4, 4, 1, 144, 4, 2, 2,
        4, 1, 32, 10, 40, 6, 2, 4, 8, 1, 9, 6, 2, 3, 46, 13, 1, 2, 198, 1, 1, 3, 1, 1, 201, 7, 1, 6,
        1, 1, 82, 22, 2, 7, 1, 2, 1, 2, 122, 6, 3, 1, 1, 2, 1, 7, 1, 1, 72, 2, 3, 1, 1, 1, 0, 2, 11,
        2, 52, 5, 5, 3, 23, 1, 0, 1, 6, 15, 0, 12, 3, 3, 0, 5, 59, 7, 0, 1, 63, 4, 81, 1, 11, 2, 0,
        2, 0, 46, 2, 23, 0, 5, 3, 6, 8, 8, 2, 7, 30, 4, 148, 3, 0, 55, 4, 50, 8, 1, 14, 1, 22, 5, 1,
        15, 0, 7, 1, 17, 2, 7, 1, 2, 1, 5, 100, 1, 160, 7, 0, 1, 61, 4, 0, 4, 254, 2, 243, 1, 2, 1,
        7, 2, 5, 1, 0, 7, 109, 7, 0, 96, 128, 240, 0,
    ];
    #[inline]
    pub fn lookup(c: char) -> bool {
        debug_assert!(!c.is_ascii());
        (c as u32) >= 0x300 && lookup_slow(c)
    }

    #[inline(never)]
    fn lookup_slow(c: char) -> bool {
        unsafe { super::skip_search(c, &SHORT_OFFSET_RUNS, &OFFSETS) }
    }
}


/// `char::to_uppercase`: the `conversions` module of the same file, reduced to the uppercase table. The
/// `L1Lut` machinery and `UPPERCASE_LUT` are untouched.
pub mod conversions {


    use crate::ops::RangeInclusive;

    struct L1Lut {
        l2_luts: [L2Lut; 2],
    }

    struct L2Lut {
        singles: &'static [(Range, i16)],
        multis: &'static [(u16, [u16; 3])],
    }

    #[derive(Copy, Clone)]
    struct Range {
        start: u16,
        len: u8,
        parity: bool,
    }

    impl Range {
        const fn new(range: RangeInclusive<u16>, parity: bool) -> Self {
            let start = *range.start();
            let end = *range.end();
            assert!(start <= end);

            let len = end - start;
            assert!(len <= 255);

            Self { start, len: len as u8, parity }
        }

        const fn singleton(start: u16) -> Self {
            Self::new(start..=start, false)
        }

        const fn step_by_1(range: RangeInclusive<u16>) -> Self {
            Self::new(range, false)
        }

        const fn step_by_2(range: RangeInclusive<u16>) -> Self {
            Self::new(range, true)
        }

        const fn start(&self) -> u16 {
            self.start
        }

        const fn end(&self) -> u16 {
            self.start + self.len as u16
        }
    }

    fn deconstruct(c: char) -> (u16, u16) {
        let c = c as u32;
        let plane = (c >> 16) as u16;
        let low = c as u16;
        (plane, low)
    }

    unsafe fn reconstruct(plane: u16, low: u16) -> char {
        // SAFETY: The caller must ensure that the result is a valid `char`.
        unsafe { char::from_u32_unchecked(((plane as u32) << 16) | (low as u32)) }
    }

    fn lookup(input: char, l1_lut: &L1Lut) -> Option<[char; 3]> {
        let (input_high, input_low) = deconstruct(input);
        let Some(l2_lut) = l1_lut.l2_luts.get(input_high as usize) else {
            return None;
        };

        let idx = l2_lut.singles.binary_search_by(|(range, _)| {
            use crate::cmp::Ordering;

            if input_low < range.start() {
                Ordering::Greater
            } else if input_low > range.end() {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        });

        if let Ok(idx) = idx {
            // SAFETY: binary search guarantees that the index is in bounds.
            let &(range, output_delta) = unsafe { l2_lut.singles.get_unchecked(idx) };
            let mask = range.parity as u16;
            if input_low & mask == range.start() & mask {
                let output_low = input_low.wrapping_add(output_delta as u16);
                // SAFETY: Table data are guaranteed to be valid Unicode.
                let output = unsafe { reconstruct(input_high, output_low) };
                return Some([output, '\0', '\0']);
            }
        };

        if let Ok(idx) = l2_lut.multis.binary_search_by_key(&input_low, |&(p, _)| p) {
            // SAFETY: binary search guarantees that the index is in bounds.
            let &(_, output_lows) = unsafe { l2_lut.multis.get_unchecked(idx) };
            // SAFETY: Table data are guaranteed to be valid Unicode.
            let output = output_lows.map(|output_low| unsafe { reconstruct(input_high, output_low) });
            return Some(output);
        };

        None
    }

    pub fn to_upper(c: char) -> [char; 3] {
        // https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=[:Changes_When_Uppercased:]-[:ASCII:]&abb=on
        if c < '\u{B5}' {
            return [c.to_ascii_uppercase(), '\0', '\0'];
        }

        lookup(c, &UPPERCASE_LUT).unwrap_or([c, '\0', '\0'])
    }


    static UPPERCASE_LUT: L1Lut = L1Lut {
        l2_luts: [
            L2Lut {
                singles: &[ // 185 entries, 1110 bytes
                    (Range::singleton(0x00b5), 743), (Range::step_by_1(0x00e0..=0x00f6), -32),
                    (Range::step_by_1(0x00f8..=0x00fe), -32), (Range::singleton(0x00ff), 121),
                    (Range::step_by_2(0x0101..=0x012f), -1), (Range::singleton(0x0131), -232),
                    (Range::step_by_2(0x0133..=0x0137), -1), (Range::step_by_2(0x013a..=0x0148), -1),
                    (Range::step_by_2(0x014b..=0x0177), -1), (Range::step_by_2(0x017a..=0x017e), -1),
                    (Range::singleton(0x017f), -300), (Range::singleton(0x0180), 195),
                    (Range::step_by_2(0x0183..=0x0185), -1), (Range::singleton(0x0188), -1),
                    (Range::singleton(0x018c), -1), (Range::singleton(0x0192), -1),
                    (Range::singleton(0x0195), 97), (Range::singleton(0x0199), -1),
                    (Range::singleton(0x019a), 163), (Range::singleton(0x019b), -22975),
                    (Range::singleton(0x019e), 130), (Range::step_by_2(0x01a1..=0x01a5), -1),
                    (Range::singleton(0x01a8), -1), (Range::singleton(0x01ad), -1),
                    (Range::singleton(0x01b0), -1), (Range::step_by_2(0x01b4..=0x01b6), -1),
                    (Range::singleton(0x01b9), -1), (Range::singleton(0x01bd), -1),
                    (Range::singleton(0x01bf), 56), (Range::singleton(0x01c5), -1),
                    (Range::singleton(0x01c6), -2), (Range::singleton(0x01c8), -1),
                    (Range::singleton(0x01c9), -2), (Range::singleton(0x01cb), -1),
                    (Range::singleton(0x01cc), -2), (Range::step_by_2(0x01ce..=0x01dc), -1),
                    (Range::singleton(0x01dd), -79), (Range::step_by_2(0x01df..=0x01ef), -1),
                    (Range::singleton(0x01f2), -1), (Range::singleton(0x01f3), -2),
                    (Range::singleton(0x01f5), -1), (Range::step_by_2(0x01f9..=0x021f), -1),
                    (Range::step_by_2(0x0223..=0x0233), -1), (Range::singleton(0x023c), -1),
                    (Range::step_by_1(0x023f..=0x0240), 10815), (Range::singleton(0x0242), -1),
                    (Range::step_by_2(0x0247..=0x024f), -1), (Range::singleton(0x0250), 10783),
                    (Range::singleton(0x0251), 10780), (Range::singleton(0x0252), 10782),
                    (Range::singleton(0x0253), -210), (Range::singleton(0x0254), -206),
                    (Range::step_by_1(0x0256..=0x0257), -205), (Range::singleton(0x0259), -202),
                    (Range::singleton(0x025b), -203), (Range::singleton(0x025c), -23217),
                    (Range::singleton(0x0260), -205), (Range::singleton(0x0261), -23221),
                    (Range::singleton(0x0263), -207), (Range::singleton(0x0264), -23193),
                    (Range::singleton(0x0265), -23256), (Range::singleton(0x0266), -23228),
                    (Range::singleton(0x0268), -209), (Range::singleton(0x0269), -211),
                    (Range::singleton(0x026a), -23228), (Range::singleton(0x026b), 10743),
                    (Range::singleton(0x026c), -23231), (Range::singleton(0x026f), -211),
                    (Range::singleton(0x0271), 10749), (Range::singleton(0x0272), -213),
                    (Range::singleton(0x0275), -214), (Range::singleton(0x027d), 10727),
                    (Range::singleton(0x0280), -218), (Range::singleton(0x0282), -23229),
                    (Range::singleton(0x0283), -218), (Range::singleton(0x0287), -23254),
                    (Range::singleton(0x0288), -218), (Range::singleton(0x0289), -69),
                    (Range::step_by_1(0x028a..=0x028b), -217), (Range::singleton(0x028c), -71),
                    (Range::singleton(0x0292), -219), (Range::singleton(0x029d), -23275),
                    (Range::singleton(0x029e), -23278), (Range::singleton(0x0345), 84),
                    (Range::step_by_2(0x0371..=0x0373), -1), (Range::singleton(0x0377), -1),
                    (Range::step_by_1(0x037b..=0x037d), 130), (Range::singleton(0x03ac), -38),
                    (Range::step_by_1(0x03ad..=0x03af), -37), (Range::step_by_1(0x03b1..=0x03c1), -32),
                    (Range::singleton(0x03c2), -31), (Range::step_by_1(0x03c3..=0x03cb), -32),
                    (Range::singleton(0x03cc), -64), (Range::step_by_1(0x03cd..=0x03ce), -63),
                    (Range::singleton(0x03d0), -62), (Range::singleton(0x03d1), -57),
                    (Range::singleton(0x03d5), -47), (Range::singleton(0x03d6), -54),
                    (Range::singleton(0x03d7), -8), (Range::step_by_2(0x03d9..=0x03ef), -1),
                    (Range::singleton(0x03f0), -86), (Range::singleton(0x03f1), -80),
                    (Range::singleton(0x03f2), 7), (Range::singleton(0x03f3), -116),
                    (Range::singleton(0x03f5), -96), (Range::singleton(0x03f8), -1),
                    (Range::singleton(0x03fb), -1), (Range::step_by_1(0x0430..=0x044f), -32),
                    (Range::step_by_1(0x0450..=0x045f), -80), (Range::step_by_2(0x0461..=0x0481), -1),
                    (Range::step_by_2(0x048b..=0x04bf), -1), (Range::step_by_2(0x04c2..=0x04ce), -1),
                    (Range::singleton(0x04cf), -15), (Range::step_by_2(0x04d1..=0x052f), -1),
                    (Range::step_by_1(0x0561..=0x0586), -48), (Range::step_by_1(0x10d0..=0x10fa), 3008),
                    (Range::step_by_1(0x10fd..=0x10ff), 3008), (Range::step_by_1(0x13f8..=0x13fd), -8),
                    (Range::singleton(0x1c80), -6254), (Range::singleton(0x1c81), -6253),
                    (Range::singleton(0x1c82), -6244), (Range::step_by_1(0x1c83..=0x1c84), -6242),
                    (Range::singleton(0x1c85), -6243), (Range::singleton(0x1c86), -6236),
                    (Range::singleton(0x1c87), -6181), (Range::singleton(0x1c88), -30270),
                    (Range::singleton(0x1c8a), -1), (Range::singleton(0x1d79), -30204),
                    (Range::singleton(0x1d7d), 3814), (Range::singleton(0x1d8e), -30152),
                    (Range::step_by_2(0x1e01..=0x1e95), -1), (Range::singleton(0x1e9b), -59),
                    (Range::step_by_2(0x1ea1..=0x1eff), -1), (Range::step_by_1(0x1f00..=0x1f07), 8),
                    (Range::step_by_1(0x1f10..=0x1f15), 8), (Range::step_by_1(0x1f20..=0x1f27), 8),
                    (Range::step_by_1(0x1f30..=0x1f37), 8), (Range::step_by_1(0x1f40..=0x1f45), 8),
                    (Range::step_by_2(0x1f51..=0x1f57), 8), (Range::step_by_1(0x1f60..=0x1f67), 8),
                    (Range::step_by_1(0x1f70..=0x1f71), 74), (Range::step_by_1(0x1f72..=0x1f75), 86),
                    (Range::step_by_1(0x1f76..=0x1f77), 100), (Range::step_by_1(0x1f78..=0x1f79), 128),
                    (Range::step_by_1(0x1f7a..=0x1f7b), 112), (Range::step_by_1(0x1f7c..=0x1f7d), 126),
                    (Range::step_by_1(0x1fb0..=0x1fb1), 8), (Range::singleton(0x1fbe), -7205),
                    (Range::step_by_1(0x1fd0..=0x1fd1), 8), (Range::step_by_1(0x1fe0..=0x1fe1), 8),
                    (Range::singleton(0x1fe5), 7), (Range::singleton(0x214e), -28),
                    (Range::step_by_1(0x2170..=0x217f), -16), (Range::singleton(0x2184), -1),
                    (Range::step_by_1(0x24d0..=0x24e9), -26), (Range::step_by_1(0x2c30..=0x2c5f), -48),
                    (Range::singleton(0x2c61), -1), (Range::singleton(0x2c65), -10795),
                    (Range::singleton(0x2c66), -10792), (Range::step_by_2(0x2c68..=0x2c6c), -1),
                    (Range::singleton(0x2c73), -1), (Range::singleton(0x2c76), -1),
                    (Range::step_by_2(0x2c81..=0x2ce3), -1), (Range::step_by_2(0x2cec..=0x2cee), -1),
                    (Range::singleton(0x2cf3), -1), (Range::step_by_1(0x2d00..=0x2d25), -7264),
                    (Range::singleton(0x2d27), -7264), (Range::singleton(0x2d2d), -7264),
                    (Range::step_by_2(0xa641..=0xa66d), -1), (Range::step_by_2(0xa681..=0xa69b), -1),
                    (Range::step_by_2(0xa723..=0xa72f), -1), (Range::step_by_2(0xa733..=0xa76f), -1),
                    (Range::step_by_2(0xa77a..=0xa77c), -1), (Range::step_by_2(0xa77f..=0xa787), -1),
                    (Range::singleton(0xa78c), -1), (Range::step_by_2(0xa791..=0xa793), -1),
                    (Range::singleton(0xa794), 48), (Range::step_by_2(0xa797..=0xa7a9), -1),
                    (Range::step_by_2(0xa7b5..=0xa7c3), -1), (Range::step_by_2(0xa7c8..=0xa7ca), -1),
                    (Range::step_by_2(0xa7cd..=0xa7db), -1), (Range::singleton(0xa7f6), -1),
                    (Range::singleton(0xab53), -928), (Range::step_by_1(0xab70..=0xabbf), 26672),
                    (Range::step_by_1(0xff41..=0xff5a), -32),
                ],
                multis: &[ // 102 entries, 816 bytes
                    (0x00df, [0x0053, 0x0053, 0x0000]), (0x0149, [0x02bc, 0x004e, 0x0000]),
                    (0x01f0, [0x004a, 0x030c, 0x0000]), (0x0390, [0x0399, 0x0308, 0x0301]),
                    (0x03b0, [0x03a5, 0x0308, 0x0301]), (0x0587, [0x0535, 0x0552, 0x0000]),
                    (0x1e96, [0x0048, 0x0331, 0x0000]), (0x1e97, [0x0054, 0x0308, 0x0000]),
                    (0x1e98, [0x0057, 0x030a, 0x0000]), (0x1e99, [0x0059, 0x030a, 0x0000]),
                    (0x1e9a, [0x0041, 0x02be, 0x0000]), (0x1f50, [0x03a5, 0x0313, 0x0000]),
                    (0x1f52, [0x03a5, 0x0313, 0x0300]), (0x1f54, [0x03a5, 0x0313, 0x0301]),
                    (0x1f56, [0x03a5, 0x0313, 0x0342]), (0x1f80, [0x1f08, 0x0399, 0x0000]),
                    (0x1f81, [0x1f09, 0x0399, 0x0000]), (0x1f82, [0x1f0a, 0x0399, 0x0000]),
                    (0x1f83, [0x1f0b, 0x0399, 0x0000]), (0x1f84, [0x1f0c, 0x0399, 0x0000]),
                    (0x1f85, [0x1f0d, 0x0399, 0x0000]), (0x1f86, [0x1f0e, 0x0399, 0x0000]),
                    (0x1f87, [0x1f0f, 0x0399, 0x0000]), (0x1f88, [0x1f08, 0x0399, 0x0000]),
                    (0x1f89, [0x1f09, 0x0399, 0x0000]), (0x1f8a, [0x1f0a, 0x0399, 0x0000]),
                    (0x1f8b, [0x1f0b, 0x0399, 0x0000]), (0x1f8c, [0x1f0c, 0x0399, 0x0000]),
                    (0x1f8d, [0x1f0d, 0x0399, 0x0000]), (0x1f8e, [0x1f0e, 0x0399, 0x0000]),
                    (0x1f8f, [0x1f0f, 0x0399, 0x0000]), (0x1f90, [0x1f28, 0x0399, 0x0000]),
                    (0x1f91, [0x1f29, 0x0399, 0x0000]), (0x1f92, [0x1f2a, 0x0399, 0x0000]),
                    (0x1f93, [0x1f2b, 0x0399, 0x0000]), (0x1f94, [0x1f2c, 0x0399, 0x0000]),
                    (0x1f95, [0x1f2d, 0x0399, 0x0000]), (0x1f96, [0x1f2e, 0x0399, 0x0000]),
                    (0x1f97, [0x1f2f, 0x0399, 0x0000]), (0x1f98, [0x1f28, 0x0399, 0x0000]),
                    (0x1f99, [0x1f29, 0x0399, 0x0000]), (0x1f9a, [0x1f2a, 0x0399, 0x0000]),
                    (0x1f9b, [0x1f2b, 0x0399, 0x0000]), (0x1f9c, [0x1f2c, 0x0399, 0x0000]),
                    (0x1f9d, [0x1f2d, 0x0399, 0x0000]), (0x1f9e, [0x1f2e, 0x0399, 0x0000]),
                    (0x1f9f, [0x1f2f, 0x0399, 0x0000]), (0x1fa0, [0x1f68, 0x0399, 0x0000]),
                    (0x1fa1, [0x1f69, 0x0399, 0x0000]), (0x1fa2, [0x1f6a, 0x0399, 0x0000]),
                    (0x1fa3, [0x1f6b, 0x0399, 0x0000]), (0x1fa4, [0x1f6c, 0x0399, 0x0000]),
                    (0x1fa5, [0x1f6d, 0x0399, 0x0000]), (0x1fa6, [0x1f6e, 0x0399, 0x0000]),
                    (0x1fa7, [0x1f6f, 0x0399, 0x0000]), (0x1fa8, [0x1f68, 0x0399, 0x0000]),
                    (0x1fa9, [0x1f69, 0x0399, 0x0000]), (0x1faa, [0x1f6a, 0x0399, 0x0000]),
                    (0x1fab, [0x1f6b, 0x0399, 0x0000]), (0x1fac, [0x1f6c, 0x0399, 0x0000]),
                    (0x1fad, [0x1f6d, 0x0399, 0x0000]), (0x1fae, [0x1f6e, 0x0399, 0x0000]),
                    (0x1faf, [0x1f6f, 0x0399, 0x0000]), (0x1fb2, [0x1fba, 0x0399, 0x0000]),
                    (0x1fb3, [0x0391, 0x0399, 0x0000]), (0x1fb4, [0x0386, 0x0399, 0x0000]),
                    (0x1fb6, [0x0391, 0x0342, 0x0000]), (0x1fb7, [0x0391, 0x0342, 0x0399]),
                    (0x1fbc, [0x0391, 0x0399, 0x0000]), (0x1fc2, [0x1fca, 0x0399, 0x0000]),
                    (0x1fc3, [0x0397, 0x0399, 0x0000]), (0x1fc4, [0x0389, 0x0399, 0x0000]),
                    (0x1fc6, [0x0397, 0x0342, 0x0000]), (0x1fc7, [0x0397, 0x0342, 0x0399]),
                    (0x1fcc, [0x0397, 0x0399, 0x0000]), (0x1fd2, [0x0399, 0x0308, 0x0300]),
                    (0x1fd3, [0x0399, 0x0308, 0x0301]), (0x1fd6, [0x0399, 0x0342, 0x0000]),
                    (0x1fd7, [0x0399, 0x0308, 0x0342]), (0x1fe2, [0x03a5, 0x0308, 0x0300]),
                    (0x1fe3, [0x03a5, 0x0308, 0x0301]), (0x1fe4, [0x03a1, 0x0313, 0x0000]),
                    (0x1fe6, [0x03a5, 0x0342, 0x0000]), (0x1fe7, [0x03a5, 0x0308, 0x0342]),
                    (0x1ff2, [0x1ffa, 0x0399, 0x0000]), (0x1ff3, [0x03a9, 0x0399, 0x0000]),
                    (0x1ff4, [0x038f, 0x0399, 0x0000]), (0x1ff6, [0x03a9, 0x0342, 0x0000]),
                    (0x1ff7, [0x03a9, 0x0342, 0x0399]), (0x1ffc, [0x03a9, 0x0399, 0x0000]),
                    (0xfb00, [0x0046, 0x0046, 0x0000]), (0xfb01, [0x0046, 0x0049, 0x0000]),
                    (0xfb02, [0x0046, 0x004c, 0x0000]), (0xfb03, [0x0046, 0x0046, 0x0049]),
                    (0xfb04, [0x0046, 0x0046, 0x004c]), (0xfb05, [0x0053, 0x0054, 0x0000]),
                    (0xfb06, [0x0053, 0x0054, 0x0000]), (0xfb13, [0x0544, 0x0546, 0x0000]),
                    (0xfb14, [0x0544, 0x0535, 0x0000]), (0xfb15, [0x0544, 0x053b, 0x0000]),
                    (0xfb16, [0x054e, 0x0546, 0x0000]), (0xfb17, [0x0544, 0x053d, 0x0000]),
                ],
            },
            L2Lut {
                singles: &[ // 12 entries, 72 bytes
                    (Range::step_by_1(0x0428..=0x044f), -40), (Range::step_by_1(0x04d8..=0x04fb), -40),
                    (Range::step_by_1(0x0597..=0x05a1), -39), (Range::step_by_1(0x05a3..=0x05b1), -39),
                    (Range::step_by_1(0x05b3..=0x05b9), -39), (Range::step_by_1(0x05bb..=0x05bc), -39),
                    (Range::step_by_1(0x0cc0..=0x0cf2), -64), (Range::step_by_1(0x0d70..=0x0d85), -32),
                    (Range::step_by_1(0x18c0..=0x18df), -32), (Range::step_by_1(0x6e60..=0x6e7f), -32),
                    (Range::step_by_1(0x6ebb..=0x6ed3), -27), (Range::step_by_1(0xe922..=0xe943), -34),
                ],
                multis: &[ // 0 entries, 0 bytes
                ],
            },
        ],
    };
}
