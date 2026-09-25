function Card({ title }: { title: string }) {
  return <article className="card">{`{"title": "${title}"}`}</article>;
}
