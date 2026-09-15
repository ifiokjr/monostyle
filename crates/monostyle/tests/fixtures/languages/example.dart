/// Validates a wallet address before submission.
///
/// Exists separately from the caller so the failure cases can be tested without
/// constructing a full transaction.
String? validateAddress(String address) {
  if (address.isEmpty) {
    return 'Address is required';
  }

  if (!address.startsWith('0x')) {
    return 'Address must be hex-encoded';
  }

  return null;
}
