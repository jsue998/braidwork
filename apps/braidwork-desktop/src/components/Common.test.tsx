import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ContentPreview, ErrorNotice } from "./Common";
it("renders model-produced HTML as inert, escaped text", () => {
  const html = renderToStaticMarkup(
    <ContentPreview
      text={'<script>alert("x")</script><img src=x onerror=alert(1)>'}
    />,
  );
  expect(html).toContain("&lt;script&gt;");
  expect(html).not.toContain("<script>");
  expect(html).not.toContain("<img");
});
it("preserves result whitespace and exact readable text", () => {
  const html = renderToStaticMarkup(
    <ContentPreview text={"Line one\n\n  Line two"} />,
  );
  expect(html).toContain("Line one\n\n  Line two");
  expect(html).toContain("<pre");
});
it("exposes human error text separately from expandable diagnostic details", () => {
  const html = renderToStaticMarkup(
    <ErrorNotice
      error={{
        code: "delegation_denied",
        message: "That agent cannot delegate more work.",
        detail: "<untrusted diagnostic>",
      }}
    />,
  );
  expect(html).toContain('role="alert"');
  expect(html).toContain("Technical details");
  expect(html).toContain("&lt;untrusted diagnostic&gt;");
});
