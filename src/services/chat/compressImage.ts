/** Max long edge for vision API payloads (keeps requests under common gateway limits). */
const MAX_EDGE = 1568;
const JPEG_QUALITY = 0.85;

/**
 * Downscale and re-encode pasted images as JPEG data URLs.
 * Large PNG screenshots often trigger proxy 502s when sent as raw base64.
 */
export function compressImageDataUrl(dataUrl: string): Promise<string> {
  return new Promise((resolve) => {
    const image = new Image();
    image.onload = () => {
      const { width, height } = image;
      if (!width || !height) {
        resolve(dataUrl);
        return;
      }

      const longEdge = Math.max(width, height);
      const scale = longEdge > MAX_EDGE ? MAX_EDGE / longEdge : 1;
      const targetW = Math.max(1, Math.round(width * scale));
      const targetH = Math.max(1, Math.round(height * scale));

      // Already small JPEG — keep as-is when no resize needed.
      if (scale === 1 && dataUrl.startsWith("data:image/jpeg")) {
        resolve(dataUrl);
        return;
      }

      const canvas = document.createElement("canvas");
      canvas.width = targetW;
      canvas.height = targetH;
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        resolve(dataUrl);
        return;
      }
      ctx.drawImage(image, 0, 0, targetW, targetH);
      try {
        resolve(canvas.toDataURL("image/jpeg", JPEG_QUALITY));
      } catch {
        resolve(dataUrl);
      }
    };
    image.onerror = () => resolve(dataUrl);
    image.src = dataUrl;
  });
}
