export async function fetcher(url: string) {
  const res = await fetch(`${process.env.NEXT_PUBLIC_API_BASE_URL}${url}`, {
    credentials: 'include',
  })
  if (res.status !== 200) {
    throw new Error("Couldn't fetch")
  }

  const json = await res.json()
  return json
}
