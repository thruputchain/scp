//! Pure heap_v2 to uncompressed profile.proto conversion, matching jeprof.
use std::{collections::BTreeMap, io};

/// One resolved frame, including an optional source position.
#[derive(Debug)]
pub struct Frame {
    pub name: String,
    pub system_name: String,
    pub file: Option<String>,
    pub line: Option<u32>,
}

/// Resolve addresses and mapped binaries from the originating process.
pub trait Symbolize {
    /// Frames for `pc`, innermost (inlined) first; empty when unknown.
    fn frames(&mut self, pc: u64) -> Vec<Frame>;
    /// Hex GNU build ID for a mapped file path, if any.
    fn build_id(&mut self, path: &str) -> Option<String>;
}

#[derive(Default)]
struct Strings {
    values: Vec<String>,
    indices: BTreeMap<String, u64>,
}

impl Strings {
    fn intern(&mut self, value: &str) -> u64 {
        if let Some(index) = self.indices.get(value) {
            return *index;
        }
        let index = self.values.len() as u64;
        self.values.push(value.to_owned());
        self.indices.insert(value.to_owned(), index);
        index
    }
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "malformed heap profile or maps")
}

fn number(text: &str, radix: u32) -> io::Result<u64> {
    let text = if radix == 16 {
        text.strip_prefix("0x").unwrap_or(text)
    } else {
        text
    };
    if text.is_empty() || !text.chars().all(|c| c.is_digit(radix)) {
        return Err(invalid());
    }
    u64::from_str_radix(text, radix).map_err(|_| invalid())
}

fn varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 128 {
        out.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    out.push(value as u8);
}
fn integer(out: &mut Vec<u8>, field: u64, value: u64) {
    varint(out, field << 3);
    varint(out, value);
}
fn message(out: &mut Vec<u8>, field: u64, value: &[u8]) {
    varint(out, (field << 3) | 2);
    varint(out, value.len() as u64);
    out.extend_from_slice(value);
}

struct Mapping {
    start: u64,
    limit: u64,
    offset: u64,
    filename: u64,
    build_id: u64,
    flags: [bool; 4],
}

fn rounded(value: f64) -> io::Result<u64> {
    if !value.is_finite() || value >= i64::MAX as f64 {
        return Err(invalid());
    }
    Ok(value.round() as u64)
}

pub fn jemalloc_to_pprof(
    dump: &str,
    maps: &str,
    symbolizer: &mut impl Symbolize,
) -> io::Result<Vec<u8>> {
    let mut lines = dump.lines();
    let period = number(
        lines
            .next()
            .and_then(|s| s.strip_prefix("heap_v2/"))
            .ok_or_else(invalid)?,
        10,
    )?;
    if period == 0 {
        return Err(invalid());
    }
    let mut strings = Strings::default();
    for value in ["", "inuse_objects", "count", "inuse_space", "bytes"] {
        strings.intern(value);
    }
    let mut build_ids = BTreeMap::new();
    let mut mappings = Vec::new();
    for line in maps.lines() {
        // splitn would lose filenames containing spaces; consume five tokens only.
        let mut rest = line;
        let mut fields = Vec::new();
        for _ in 0..5 {
            rest = rest.trim_start();
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            if end == 0 {
                return Err(invalid());
            }
            fields.push(&rest[..end]);
            rest = &rest[end..];
        }
        let (start, limit) = fields[0].split_once('-').ok_or_else(invalid)?;
        let start = number(start, 16)?;
        let limit = number(limit, 16)?;
        if start >= limit {
            return Err(invalid());
        }
        let path = rest.trim_start();
        let filename = strings.intern(path);
        let build_id = if path.starts_with('/') {
            *build_ids.entry(path).or_insert_with(|| {
                symbolizer
                    .build_id(path)
                    .map_or(0, |id| strings.intern(&id))
            })
        } else {
            0
        };
        mappings.push(Mapping {
            start,
            limit,
            offset: number(fields[2], 16)?,
            filename,
            build_id,
            flags: [false; 4],
        });
    }
    let mut profile = Vec::new();
    for (typ, unit) in [(1, 2), (3, 4)] {
        let mut value = Vec::new();
        integer(&mut value, 1, typ);
        integer(&mut value, 2, unit);
        message(&mut profile, 1, &value);
    }
    let mut locations = BTreeMap::new();
    let mut stack = Vec::new();
    let mut consumed = true;
    for line in lines {
        if line == "MAPPED_LIBRARIES:" {
            break;
        }
        if let Some(frames) = line.strip_prefix("@ ") {
            if !consumed {
                return Err(invalid());
            }
            stack.clear();
            consumed = false;
            for address in frames.split_whitespace() {
                let mut pc = number(address, 16)?;
                if pc == 0 {
                    return Err(invalid());
                }
                if !stack.is_empty() {
                    pc -= 1;
                }
                let next = locations.len() as u64 + 1;
                stack.push(*locations.entry(pc).or_insert(next));
            }
            if stack.is_empty() {
                return Err(invalid());
            }
        } else {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.first() != Some(&"t*:") || stack.is_empty() {
                continue;
            }
            if consumed || fields.len() != 5 {
                return Err(invalid());
            }
            let n =
                number(fields[1].strip_suffix(':').ok_or_else(invalid)?, 10)?;
            let b = number(fields[2], 10)?;
            number(
                fields[3]
                    .strip_prefix('[')
                    .and_then(|s| s.strip_suffix(':'))
                    .ok_or_else(invalid)?,
                10,
            )?;
            number(fields[4].strip_suffix(']').ok_or_else(invalid)?, 10)?;
            consumed = true;
            if (n == 0) != (b == 0) {
                return Err(invalid());
            }
            if n == 0 {
                continue;
            }
            let probability =
                -(-(b as f64) / n as f64 / period as f64).exp_m1();
            let bytes = rounded(b as f64 / probability)?;
            let count = rounded(bytes as f64 * n as f64 / b as f64)?;
            let mut sample = Vec::new();
            for id in &stack {
                integer(&mut sample, 1, *id);
            }
            integer(&mut sample, 2, count);
            integer(&mut sample, 2, bytes);
            message(&mut profile, 2, &sample);
        }
    }
    if !consumed {
        return Err(invalid());
    }
    let mut functions = BTreeMap::new();
    for (pc, id) in locations {
        let mut location = Vec::new();
        integer(&mut location, 1, id);
        let mapping = mappings
            .iter_mut()
            .enumerate()
            .find(|(_, m)| pc >= m.start && pc < m.limit);
        let frames = symbolizer.frames(pc);
        if let Some((index, m)) = mapping {
            integer(&mut location, 2, index as u64 + 1);
            m.flags[0] |= !frames.is_empty();
            m.flags[1] |= frames.iter().any(|f| f.file.is_some());
            m.flags[2] |= frames.iter().any(|f| f.line.is_some());
            m.flags[3] |= frames.len() > 1;
        }
        integer(&mut location, 3, pc);
        for frame in frames {
            let key = (frame.name, frame.system_name, frame.file);
            let next = functions.len() as u64 + 1;
            let function_id = *functions.entry(key).or_insert(next);
            let mut line = Vec::new();
            integer(&mut line, 1, function_id);
            if let Some(number) = frame.line {
                integer(&mut line, 2, u64::from(number));
            }
            message(&mut location, 4, &line);
        }
        message(&mut profile, 4, &location);
    }
    for ((name, system_name, file), id) in functions {
        let mut function = Vec::new();
        integer(&mut function, 1, id);
        integer(&mut function, 2, strings.intern(&name));
        integer(&mut function, 3, strings.intern(&system_name));
        if let Some(file) = file {
            integer(&mut function, 4, strings.intern(&file));
        }
        message(&mut profile, 5, &function);
    }
    for (index, m) in mappings.iter().enumerate() {
        let mut encoded = Vec::new();
        for (field, value) in [
            (1, index as u64 + 1),
            (2, m.start),
            (3, m.limit),
            (4, m.offset),
            (5, m.filename),
        ] {
            integer(&mut encoded, field, value);
        }
        if m.build_id != 0 {
            integer(&mut encoded, 6, m.build_id);
        }
        for (index, flag) in m.flags.iter().enumerate() {
            if *flag {
                integer(&mut encoded, 7 + index as u64, 1);
            }
        }
        message(&mut profile, 3, &encoded);
    }
    for value in strings.values {
        message(&mut profile, 6, value.as_bytes());
    }
    integer(&mut profile, 14, 3);
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::wire::decode;
    use super::*;

    struct Unknown;
    impl Symbolize for Unknown {
        fn frames(&mut self, _: u64) -> Vec<Frame> {
            Vec::new()
        }
        fn build_id(&mut self, _: &str) -> Option<String> {
            None
        }
    }

    #[test]
    fn captured_fixture() {
        let profile = jemalloc_to_pprof(
            include_str!("../testdata/jemalloc.heap"),
            "",
            &mut Unknown,
        )
        .unwrap();
        let fields = decode(&profile);
        let samples: Vec<_> = fields.iter().filter(|f| f.0 == 2).collect();
        assert!(!samples.is_empty());
        for sample in samples {
            assert!(!decode(sample.2).is_empty());
        }
    }
    #[test]
    fn sampling_locations_and_mappings() {
        let profile = jemalloc_to_pprof("heap_v2/524288\n  t*: 99: 999 [0: 0]\n@ 0x1011 0x1021\n  t*: 1: 524288 [0: 0]\n  t0: 1: 524288 [0: 0]\n@ 0x1011\n  t*: 0: 0 [0: 0]\nMAPPED_LIBRARIES:\n", "1000-2000 r-xp 00001000 00:00 1 /tmp/a binary\n", &mut Unknown).unwrap();
        let fields = decode(&profile);
        for (tag, expected) in [(1, 2), (2, 1), (3, 1), (4, 2), (6, 6)] {
            assert_eq!(fields.iter().filter(|f| f.0 == tag).count(), expected);
        }
        let sample = decode(fields.iter().find(|f| f.0 == 2).unwrap().2);
        assert_eq!(
            sample.iter().map(|f| (f.0, f.1)).collect::<Vec<_>>(),
            [(1, 1), (1, 2), (2, 2), (2, 829411)]
        );
        let mapping = decode(fields.iter().find(|f| f.0 == 3).unwrap().2);
        assert_eq!(
            mapping.iter().map(|f| f.1).collect::<Vec<_>>(),
            [1, 0x1000, 0x2000, 0x1000, 5]
        );
        for field in fields.iter().filter(|f| f.0 == 4) {
            let location = decode(field.2);
            assert_eq!(location.len(), 3);
            assert_eq!(location[1].1, 1);
            assert!([0x1011, 0x1020].contains(&location[2].1));
        }
        let strings: Vec<_> =
            fields.iter().filter(|f| f.0 == 6).map(|f| f.2).collect();
        assert_eq!(strings[0], b"");
        assert_eq!(strings[5], b"/tmp/a binary");
        assert!(!fields.iter().any(|f| f.0 == 5));
    }
    #[test]
    fn symbols_inline_order_and_build_ids() {
        struct Fake {
            calls: usize,
            pcs: Vec<u64>,
        }
        impl Symbolize for Fake {
            fn frames(&mut self, pc: u64) -> Vec<Frame> {
                self.pcs.push(pc);
                let frame = |name: &str, line| Frame {
                    name: name.to_owned(),
                    system_name: name.to_owned(),
                    file: Some("source.rs".to_owned()),
                    line: Some(line),
                };
                if pc == 0x1011 {
                    vec![frame("inner", 10), frame("outer", 20)]
                } else {
                    vec![frame("outer", 30)]
                }
            }
            fn build_id(&mut self, path: &str) -> Option<String> {
                assert_eq!(path, "/tmp/binary");
                self.calls += 1;
                Some("abcd".to_owned())
            }
        }
        let mut fake = Fake {
            calls: 0,
            pcs: Vec::new(),
        };
        let profile = jemalloc_to_pprof(
            "heap_v2/1\n@ 0x1011 0x1021\n t*: 1: 100 [0: 0]\n",
            "1000-2000 r-xp 0 00:00 1 /tmp/binary\n2000-3000 r--p 0 00:00 1 /tmp/binary\n3000-4000 rw-p 0 00:00 0 [heap]\n",
            &mut fake,
        ).unwrap();
        assert_eq!(fake.calls, 1);
        assert_eq!(fake.pcs, [0x1011, 0x1020]);
        let fields = decode(&profile);
        let strings: Vec<_> =
            fields.iter().filter(|f| f.0 == 6).map(|f| f.2).collect();
        let unique: std::collections::BTreeSet<_> = strings.iter().collect();
        assert_eq!(unique.len(), strings.len());
        assert_eq!(
            &strings[..5],
            &[
                &b""[..],
                &b"inuse_objects"[..],
                &b"count"[..],
                &b"inuse_space"[..],
                &b"bytes"[..]
            ]
        );
        let functions: Vec<_> = fields
            .iter()
            .filter(|f| f.0 == 5)
            .map(|f| decode(f.2))
            .collect();
        assert_eq!(functions.len(), 2);
        for (function, name, id) in [
            (&functions[0], &b"inner"[..], 1),
            (&functions[1], &b"outer"[..], 2),
        ] {
            assert_eq!(function[0].1, id);
            assert_eq!(strings[usize::try_from(function[1].1).unwrap()], name);
            assert_eq!(function[1].1, function[2].1);
            assert_eq!(
                strings[usize::try_from(function[3].1).unwrap()],
                b"source.rs"
            );
        }
        let locations: Vec<_> = fields
            .iter()
            .filter(|f| f.0 == 4)
            .map(|f| decode(f.2))
            .collect();
        let lines: Vec<_> = locations[0]
            .iter()
            .filter(|f| f.0 == 4)
            .map(|f| decode(f.2).iter().map(|f| (f.0, f.1)).collect::<Vec<_>>())
            .collect();
        assert_eq!(lines, [vec![(1, 1), (2, 10)], vec![(1, 2), (2, 20)]]);
        let line = decode(locations[1].iter().find(|f| f.0 == 4).unwrap().2);
        assert_eq!(
            line.iter().map(|f| (f.0, f.1)).collect::<Vec<_>>(),
            [(1, 2), (2, 30)]
        );
        let mappings: Vec<_> = fields
            .iter()
            .filter(|f| f.0 == 3)
            .map(|f| decode(f.2))
            .collect();
        for mapping in &mappings[..2] {
            let id = mapping.iter().find(|f| f.0 == 6).unwrap().1;
            assert_eq!(strings[usize::try_from(id).unwrap()], b"abcd");
            assert_eq!(mapping[4].1, mappings[0][4].1);
        }
        for tag in 7..=10 {
            assert!(mappings[0].iter().any(|f| f.0 == tag && f.1 == 1));
            assert!(!mappings[1].iter().any(|f| f.0 == tag));
        }
        assert!(!mappings[2].iter().any(|f| f.0 >= 6));
    }

    #[test]
    fn rejects_malformed_input() {
        for dump in [
            "heap_v1/1\n",
            "heap_v2/0\n",
            "heap_v2/1\n@ 0x1\n",
            "heap_v2/1\n@ 0x0\n",
            "heap_v2/1\n@ 0x1\n t*: 0: 1 [0: 0]\n",
            "heap_v2/1\n@ 0x1\n t*: 1: 1 [0: 0]\n t*: 1: 1 [0: 0]\n",
            "heap_v2/1\n@ \n",
            "heap_v2/1\n@ 0x1\n@ 0x2\n",
            "heap_v2/1\n@ 0x1\n t*: 1: 18446744073709551615 [0: 0]\n",
        ] {
            assert!(
                jemalloc_to_pprof(dump, "", &mut Unknown).is_err(),
                "{dump}"
            );
        }
        for maps in [
            "garbage",
            "2000-1000 r-xp 0 00:00 1",
            "x-2000 r-xp 0 00:00 1",
        ] {
            assert!(
                jemalloc_to_pprof("heap_v2/1\n", maps, &mut Unknown).is_err()
            );
        }
    }
}

#[cfg(test)]
pub(crate) mod wire {
    fn var(input: &mut &[u8]) -> u64 {
        let mut value = 0;
        for shift in (0..64).step_by(7) {
            let byte = input[0];
            *input = &input[1..];
            value |= u64::from(byte & 127) << shift;
            if byte & 128 == 0 {
                return value;
            }
        }
        panic!("invalid varint")
    }
    pub(crate) fn decode(mut input: &[u8]) -> Vec<(u64, u64, &[u8])> {
        let mut fields = Vec::new();
        while !input.is_empty() {
            let key = var(&mut input);
            let (value, bytes) = if key & 7 == 0 {
                (var(&mut input), &[][..])
            } else {
                assert_eq!(key & 7, 2);
                let size = var(&mut input) as usize;
                let bytes = &input[..size];
                input = &input[size..];
                (0, bytes)
            };
            fields.push((key >> 3, value, bytes));
        }
        fields
    }
}
