abstract class Cache {
  factory Cache(Map<String, int> seed) = _MapCache;

  int lookup(String key);
}

class _MapCache implements Cache {
  _MapCache(this._entries);

  @override
  int lookup(String key) => _entries[key] ?? -1;

  final Map<String, int> _entries;
}
