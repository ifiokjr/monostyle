function signedMessageOutput(message: Uint8Array, signature: Ed25519Signature): JsObject {
	const output = new JsObject();
	set(output, "signedMessage", bytesToJs(message));
	set(output, "signature", bytesToJs(signature.asRef()));

	return output;
}
