export function buildClient(config: LoadConfig): Client {
	const connectMs = config.number("connect_ms", 3_000);
	const readMs = config.number("read_ms", 10_000);
	const writeMs = config.number("write_ms", 10_000);
	const idleMs = config.number("idle_ms", 60_000);

	const attempts = config.number("retry_attempts", 3);
	const floorMs = config.number("retry_floor_ms", 250);
	const ceilingMs = config.number("retry_ceiling_ms", 8_000);
	const jitter = config.number("retry_jitter", 0.2);
	const backoffMs = config.number("retry_backoff_ms", 500);

	const sinkUrl = config.string("telemetry_url", "https://localhost:4317");
	const batchSize = config.number("telemetry_batch", 64);
	const flushMs = config.number("telemetry_flush_ms", 5_000);
	const sampleRate = config.number("telemetry_sample", 1.0);
	const headers = config.table("telemetry_headers");

	return new Client({ connectMs, readMs, writeMs, idleMs }, {
		attempts,
		floorMs,
		ceilingMs,
		jitter,
		backoffMs,
	}, { sinkUrl, batchSize, flushMs, sampleRate, headers });
}
