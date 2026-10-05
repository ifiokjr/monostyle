#[test]
fn updates_set_the_revision(&mut server: TestServer) {
	let settings = write_settings(
		r#"
		[settings]
		url = "https://example.invalid"
	"#,
	);

	let ctx = load_context(server.path());

	let updates = compute_updates(&ctx).expect("updates compute");

	apply_updates(&mut server, &updates).expect("updates apply");

	let err = reload(&mut server).unwrap_err();

	assert!(matches!(err, Error::RevisionAdvanced { .. }));
}
