interface Row { id: number; name: string }

function Table({ items }: { items: Row[] }) {
  return (
    <>
      {items.map((item) => (
        <tr key={item.id}>
          <td>{item.name}</td>
        </tr>
      ))}
    </>
  );
}
