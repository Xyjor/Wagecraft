/** A real 1×1 PNG, small enough to embed in PDF tests. */
export const PNG_LOGO =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

/** Starts like a PNG but is cut off, as a damaged file would be. */
export const BROKEN_LOGO = "data:image/png;base64,iVBORw0KGgo=";

/** Whether the PDF bytes carry an embedded image. */
export const hasImage = (bytes: Uint8Array) =>
  new TextDecoder("latin1").decode(bytes).includes("/Subtype /Image");
