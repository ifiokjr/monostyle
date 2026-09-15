//! A worked example of code that is genuinely unpleasant to read.
//!
//! Every problem this file demonstrates is one monostyle detects: no blank lines before
//! control flow, deeply nested branches, a parameter list that should be split, a long
//! unbroken statement run, and no comment explaining any of the reasoning.
//!
//! This file scores poorly for readability *and* complexity, which is the point: it is hard
//! to read and hard to test.

pub fn process(input: &Input, mode: Mode, flags: Flags, cache: &Cache) -> Result<Output, Error> {
    if input.is_empty() {
        return Err(Error::Empty);
    }
    if !input.is_valid_format() {
        return Err(Error::InvalidFormat);
    }
    if input.len() > 4096 {
        return Err(Error::TooLong);
    }
    let normalized = input.normalized();
    let expanded = normalized.expand();
    let hashed = expanded.hash();
    let cached = cache.lookup(&hashed);
    if mode == Mode::Strict {
        if flags.contains(Flags::VERIFY) {
            if !cached.is_empty() {
                if cached.verify().is_ok() {
                    if flags.contains(Flags::NORMALIZE) {
                        for entry in cached.entries() {
                            if entry.is_stale() {
                                if entry.can_refresh() {
                                    entry.refresh();
                                }
                            }
                        }
                    } else {
                        for entry in cached.entries() {
                            if entry.needs_write() {
                                entry.write();
                            }
                        }
                    }
                }
            }
        }
    }
    let a = compute(expanded, hashed, mode, flags, cache, input.scale, input.offset);
    let b = transform(a, mode, flags, cache, input.scale, input.offset, input.limit);
    if b.is_ok() && flags.contains(Flags::EMIT) && !cached.is_empty() && mode != Mode::Dry {
        emit(b, cached, input.scale, input.offset, input.limit, input.target, input.format);
    }
    Ok(b)
}
