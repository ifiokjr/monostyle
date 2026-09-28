fn build(args: &mut Args, receipts_enabled: bool) {
	args.settlement_bounty_lamports
		.set(if receipts_enabled { 1_000 } else { 0 });
	args.result_receipts_enabled.set(receipts_enabled);
}

fn verify(bundle: &Bundle, index: usize) -> Result<(), Error> {
	if index >= usize::from(bundle.asset_count)
		|| bundle.kinds[index] != 0
		|| mint_at(bundle, index)? != Address::default()
	{
		return Err(lootbox_error(LootboxError::InvalidPrizePool));
	}

	Ok(())
}

fn assert_pool(receipts_enabled: bool, total_bundles: u64) {
	builder()
		.manifest_hash(manifest_hash)
		.service_vault_bump(args.service_vault_bump)
		.result_receipt_rent_lamports(receipt_rent)
		.remaining_result_receipts(if receipts_enabled { total_bundles } else { 0 })
		.send();
}
