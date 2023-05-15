export async function generateStaticParams() {
  const ids = await fetch(
    `${process.env.NEXT_PUBLIC_API_BASE_URL}/api/apps/all_ids`,
    { next: { revalidate: 300 } }
  ).then((res) => res.json())

  return ids.map((id: string) => ({
    id,
  }))
}

export default function AppLayout({
  children,
  params: { lang, id },
}: {
  children: React.ReactNode
  params: { lang: string; id: string }
}) {
  return <>{children}</>
}
