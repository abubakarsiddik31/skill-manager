import { marked } from "marked";
import DOMPurify from "dompurify";

export function renderMarkdown(raw: string): string {
  return DOMPurify.sanitize(marked.parse(raw, { async: false }) as string);
}
