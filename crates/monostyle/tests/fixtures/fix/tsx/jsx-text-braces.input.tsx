function Banner({ count }: { count: number }) {
  return (
    <div>
      <span>{"literal {"}</span>
      <strong>{count > 3 ? "many" : "few"}</strong>
      <em>{"}"}</em>
    </div>
  );
}
