/// A snapshot command followed by a loop that reads its output through a here-string.
pub const SNAPSHOT_AND_IDENTITY: &str = r#"owned_snapshot() {
  owned_rows=$("$OWNED_TIMEOUT" -k 0.1s 0.25s /bin/ps -axo \
    pid=,ppid=,pgid=,stat=,lstart=) || return 1
}

owned_identity() {
  local pid parent group state started
  [ -n "$owned_pid" ] && [ -n "$owned_start_time" ] || return 1
  while read -r pid parent group state started; do
    if [ "$pid" = "$owned_pid" ]; then
      [ "$parent" = "$$" ] && [ "$group" = "$owned_pid" ] &&
        [ "$started" = "$owned_start_time" ] && [[ "$state" != Z* ]]
      return
    fi
  done <<< "$owned_rows"
  return 1
}

owned_next() {
  echo done
}
"#;
