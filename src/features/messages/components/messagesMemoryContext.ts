import { MEMORY_CONTEXT_SUMMARY_PREFIX } from "../../project-memory/utils/memoryMarkers";

export type MemoryContextSummary = {
  preview: string;
  lines: string[];
};

const PROJECT_MEMORY_KIND_LINE_REGEX =
  /^\[(?:已知问题|技术决策|项目上下文|对话记录|笔记|记忆)\]\s*/;
const LEGACY_MEMORY_RECORD_HINT_REGEX =
  /(?:用户输入[:：]|助手输出摘要[:：]|助手输出[:：])/;
const PROJECT_MEMORY_XML_PREFIX_REGEX =
  /^<project-memory\b[^>]*>([\s\S]*?)<\/project-memory>\s*/i;
const PARAGRAPH_BREAK_SPLIT_REGEX = /\r?\n[^\S\r\n]*\r?\n+/;

function buildMemorySummary(preview: string): MemoryContextSummary | null {
  const normalizedPreview = preview.trim();
  if (!normalizedPreview) {
    return null;
  }
  const lines = normalizedPreview
    .split(/[；\n]+/)
    .map((line) => line.trim())
    .filter(Boolean);
  return {
    preview: normalizedPreview,
    lines: lines.length > 0 ? lines : [normalizedPreview],
  };
}

export function parseMemoryContextSummary(text: string): MemoryContextSummary | null {
  const normalized = text.trim();
  if (!normalized.startsWith(MEMORY_CONTEXT_SUMMARY_PREFIX)) {
    return null;
  }
  const preview = normalized.slice(MEMORY_CONTEXT_SUMMARY_PREFIX.length).trim();
  if (!preview) {
    return null;
  }
  const lines = preview
    .split(/[；\n]+/)
    .map((line) => line.trim())
    .filter(Boolean);
  return {
    preview,
    lines: lines.length > 0 ? lines : [preview],
  };
}

export function parseInjectedMemoryPrefixFromUser(
  text: string,
): { memorySummary: MemoryContextSummary; remainingText: string } | null {
  const normalized = text.trimStart();
  if (!normalized) {
    return null;
  }

  const xmlMatch = normalized.match(PROJECT_MEMORY_XML_PREFIX_REGEX);
  if (xmlMatch) {
    const blockBody = (xmlMatch[1] ?? "").trim();
    const memoryLines = blockBody
      .split(/\r?\n+/)
      .map((line) => line.trim())
      .filter(Boolean)
      .filter((line) => PROJECT_MEMORY_KIND_LINE_REGEX.test(line));
    const previewText = memoryLines.length > 0 ? memoryLines.join("；") : blockBody;
    const memorySummary = buildMemorySummary(previewText);
    if (!memorySummary) {
      return null;
    }
    const remainingText = normalized.slice(xmlMatch[0].length).trimStart();
    return { memorySummary, remainingText };
  }

  if (!PROJECT_MEMORY_KIND_LINE_REGEX.test(normalized)) {
    return null;
  }
  if (!LEGACY_MEMORY_RECORD_HINT_REGEX.test(normalized)) {
    return null;
  }

  const paragraphBlocks = normalized.split(PARAGRAPH_BREAK_SPLIT_REGEX);
  if (paragraphBlocks.length >= 2) {
    const firstBlock = (paragraphBlocks[0] ?? "").trim();
    if (
      PROJECT_MEMORY_KIND_LINE_REGEX.test(firstBlock) &&
      LEGACY_MEMORY_RECORD_HINT_REGEX.test(firstBlock)
    ) {
      const memorySummary = buildMemorySummary(firstBlock);
      if (!memorySummary) {
        return null;
      }
      return {
        memorySummary,
        remainingText: paragraphBlocks.slice(1).join("\n\n").trimStart(),
      };
    }
  }

  const lines = normalized.split(/\r?\n/);
  if (lines.length >= 2) {
    const firstLine = (lines[0] ?? "").trim();
    if (
      PROJECT_MEMORY_KIND_LINE_REGEX.test(firstLine) &&
      LEGACY_MEMORY_RECORD_HINT_REGEX.test(firstLine)
    ) {
      const memorySummary = buildMemorySummary(firstLine);
      if (!memorySummary) {
        return null;
      }
      return {
        memorySummary,
        remainingText: lines.slice(1).join("\n").trimStart(),
      };
    }
  }

  return null;
}
