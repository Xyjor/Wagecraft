import { expect } from "vitest";

const inflate = (data: Uint8Array<ArrayBuffer>) =>
  new Response(
    new Blob([data]).stream().pipeThrough(new DecompressionStream("deflate")),
  ).arrayBuffer();

/**
 * Checks that bytes are a PDF a viewer can open: the header, and every page stream
 * unpacks. jsdom's Blob mangles binary bytes but keeps the header, which shows as a blank
 * page, so run PDF tests in the node environment.
 */
export async function expectReadablePdf(bytes: Uint8Array<ArrayBuffer>) {
  const text = new TextDecoder("latin1").decode(bytes);
  expect(text.slice(0, 5)).toBe("%PDF-");
  const streams = [...text.matchAll(/\/Length (\d+)[^]*?stream\r?\n/g)];
  expect(streams.length).toBeGreaterThan(0);
  for (const m of streams) {
    const start = m.index + m[0].length;
    await expect(inflate(bytes.slice(start, start + Number(m[1])))).resolves.toBeDefined();
  }
}
