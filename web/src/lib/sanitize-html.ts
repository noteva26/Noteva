const BLOCKED_TAGS = new Set(["script", "style", "object", "embed", "base", "meta", "link"]);
const URL_ATTRIBUTES = new Set(["href", "src", "xlink:href", "formaction"]);
const SAFE_PROTOCOLS = new Set(["http:", "https:", "mailto:", "tel:"]);
const SAFE_DATA_IMAGE = /^data:image\/(?:png|jpe?g|gif|webp|avif);base64,/i;

function isSafeUrl(value: string) {
  const trimmed = value.trim();
  if (!trimmed || trimmed.startsWith("#")) return true;
  if (SAFE_DATA_IMAGE.test(trimmed)) return true;

  try {
    const url = new URL(trimmed, window.location.origin);
    return SAFE_PROTOCOLS.has(url.protocol);
  } catch {
    return false;
  }
}

export function sanitizeHtml(html: string | null | undefined) {
  if (!html) return "";
  if (typeof document === "undefined") return html;

  const template = document.createElement("template");
  template.innerHTML = html;

  const blockedElements: Element[] = [];
  const walker = document.createTreeWalker(template.content, NodeFilter.SHOW_ELEMENT);

  while (walker.nextNode()) {
    const element = walker.currentNode as Element;
    const tagName = element.tagName.toLowerCase();

    if (BLOCKED_TAGS.has(tagName)) {
      blockedElements.push(element);
      continue;
    }

    for (const attr of Array.from(element.attributes)) {
      const name = attr.name.toLowerCase();
      const value = attr.value;

      if (name.startsWith("on") || name === "srcdoc") {
        element.removeAttribute(attr.name);
        continue;
      }

      if (URL_ATTRIBUTES.has(name) && !isSafeUrl(value)) {
        element.removeAttribute(attr.name);
      }
    }
  }

  blockedElements.forEach((element) => element.remove());
  return template.innerHTML;
}
