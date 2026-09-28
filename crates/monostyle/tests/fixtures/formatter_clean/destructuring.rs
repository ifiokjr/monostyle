fn kind(inner: &Type) -> Option<Kind> {
	if let Some(Segment {
		payload: Vec { elem, max, pfx },
		..
	}) = classify_vec(inner)
	{
		return Some(Kind::Segment(elem));
	}

	None
}
