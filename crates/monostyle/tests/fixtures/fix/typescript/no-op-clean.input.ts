interface User {
  id: number;
  name: string;
}

function describe(user: User): string {
  const note = `{"kept": ${true}, "name": "${user.name}"}`;

  return `${user.id}: ${note}`;
}
