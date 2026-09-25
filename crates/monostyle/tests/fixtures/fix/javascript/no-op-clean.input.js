function describe(user) {
  const note = `{"kept": ${true}, "name": "${user.name}"}`;

  return `${user.id}: ${note}`;
}
