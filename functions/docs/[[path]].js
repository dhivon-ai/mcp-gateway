// The generated _routes.json sends only unpublished documentation paths here.
export async function onRequest({ request, env }) {
  const page = await env.ASSETS.fetch(new URL("/404", request.url));
  return new Response(request.method === "HEAD" ? null : page.body, {
    status: 404,
    headers: {
      "Content-Type": "text/html; charset=utf-8",
      "Cache-Control": "no-store",
      "X-Content-Type-Options": "nosniff",
    },
  });
}
