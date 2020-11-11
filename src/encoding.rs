mod cp037;

pub trait SBCS {
    fn from_unicode(ch: char) -> Option<u8>;
    fn to_unicode(ch: u8) -> char;
}

pub fn to_cp037(stream: impl Iterator<Item=char>) -> impl Iterator<Item=u8> {
    stream.map(|ch| {
        let ch = cp037::ENCODE_TBL.get(ch as usize)
            .copied()
            .unwrap_or(0x40);
        if ch < 0x40 { // prohibit sending control codes.
            0x40
        } else {
            ch
        }
    })
}