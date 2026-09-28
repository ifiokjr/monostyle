fn work(bits: &mut Vec<u8>) {
	if bits.is_empty() {
		bits.push(0);
	}
	store(bits);
	let kept = bits.len();
	return;
}
