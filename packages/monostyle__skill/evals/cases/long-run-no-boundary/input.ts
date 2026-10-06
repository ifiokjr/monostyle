export function formatInvoice(rawLines: string[], rates: Record<string, number>): string {
	const currency = rawLines[0].trim();
	const symbol = currency === "EUR" ? "€" : currency === "GBP" ? "£" : "$";
	const items: Array<{ name: string; qty: number; unit: number }> = [];
	for (const line of rawLines.slice(1)) {
		const parts = line.split(";");
		const name = parts[0].trim();
		const qty = Number(parts[1]);
		const unit = Number(parts[2]);
		items.push({ name, qty, unit });
	}
	let subtotal = 0;
	for (const item of items) {
		subtotal += item.qty * item.unit;
	}
	const rate = rates[currency] ?? 1;
	let discount = 0;
	if (subtotal > 1000) {
		discount = subtotal * 0.05;
	} else if (subtotal > 500) {
		discount = subtotal * 0.02;
	}
	const taxable = subtotal - discount;
	const vat = taxable * 0.2;
	const total = taxable + vat;
	const width = 32;
	const rows: string[] = [];
	for (const item of items) {
		const amount = item.qty * item.unit;
		rows.push(item.name.padEnd(width - 10) + symbol + amount.toFixed(2));
	}
	const discountRow = discount > 0
		? "discount".padEnd(width - 10) + "-" + symbol + discount.toFixed(2)
		: "";
	const totalRow = "total".padEnd(width - 10) + symbol + total.toFixed(2);
	const body = rows.filter((row) => row.length > 0).join("\n");
	const tail = [discountRow, totalRow].filter((row) => row.length > 0).join("\n");
	return [body, tail].join("\n") + "\n" + `rate ${rate.toFixed(4)}`;
}
