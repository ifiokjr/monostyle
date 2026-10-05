struct Fields {
	sequence: u64,
	revision: u32,
}

fn decode_fields(plan: &mut Plan<'_>) -> Fields {
	let label_len = plan.byte() as usize % 80;
	let values_len = plan.byte() as usize % 20;
	let note_present = plan.byte() % 2 == 1;
	let note_len = plan.byte() as usize % 12;
	let mut sequence = [0u8; 8];
	let chunk = plan.take(8);
	sequence[..chunk.len()].copy_from_slice(chunk);

	let mut revision = [0u8; 4];
	let chunk = plan.take(4);
	revision[..chunk.len()].copy_from_slice(chunk);

	Fields {
		sequence: u64::from_le_bytes(sequence),
		revision: u32::from_le_bytes(revision),
		label: String::from_utf8_lossy(&plan.take(label_len)).into_owned(),
		values: plan.take(values_len).to_vec(),
		note: if note_present {
			Some(String::from_utf8_lossy(&plan.take(note_len)).into_owned())
		} else {
			None
		},
	}
}
