#!/usr/bin/env node

import { promises as fs } from "node:fs";
import path from "node:path";

const TEXT_EXTENSIONS = new Set([
  ".ts",
  ".tsx",
  ".js",
  ".jsx",
  ".rs",
  ".css",
  ".scss",
  ".vue",
  ".svelte",
  ".java",
  ".kt",
  ".go",
  ".py",
]);

const EXCLUDED_DIRS = new Set([
  ".git",
  "node_modules",
  "dist",
  "build",
  "coverage",
  "target",
  "out",
  ".next",
  ".turbo",
]);

function parseArgs(argv) {
  const config = {
    threshold: 3000,
    mode: "report",
    markdownOutput: null,
    root: process.cwd(),
  };

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (token === "--threshold") {
      const value = Number(argv[index + 1]);
      if (!Number.isFinite(value) || value <= 0) {
        throw new Error(`Invalid --threshold value: ${argv[index + 1] ?? "<missing>"}`);
      }
      config.threshold = value;
      index += 1;
      continue;
    }
    if (token === "--mode") {
      const value = argv[index + 1];
      if (!["report", "warn", "fail"].includes(value)) {
        throw new Error(`Invalid --mode value: ${value ?? "<missing>"}`);
      }
      config.mode = value;
      index += 1;
      continue;
    }
    if (token === "--markdown-output") {
      config.markdownOutput = argv[index + 1];
      if (!config.markdownOutput) {
        throw new Error("Missing value for --markdown-output");
      }
      index += 1;
      continue;
    }
    if (token === "--root") {
      const root = argv[index + 1];
      if (!root) {
        throw new Error("Missing value for --root");
      }
      config.root = path.resolve(root);
      index += 1;
      continue;
    }

    throw new Error(`Unknown argument: ${token}`);
  }

  return config;
}

async function walkDirectory(directoryPath) {
  const entries = await fs.readdir(directoryPath, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const fullPath = path.join(directoryPath, entry.name);
    if (entry.isDirectory()) {
      if (EXCLUDED_DIRS.has(entry.name)) {
        continue;
      }
      files.push(...(await walkDirectory(fullPath)));
      continue;
    }

    if (!entry.isFile()) {
      continue;
    }
    files.push(fullPath);
  }

  return files;
}

function detectType(relativePath, extension) {
  if (relativePath.startsWith("src/i18n/locales/")) {
    return "i18n";
  }
  if (relativePath.startsWith("src/styles/")) {
    return "css";
  }
  switch (extension) {
    case ".ts":
    case ".tsx":
      return "ts/tsx";
    case ".js":
    case ".jsx":
      return "js/jsx";
    case ".rs":
      return "rust";
    case ".css":
    case ".scss":
      return "css";
    default:
      return extension.slice(1);
  }
}

function detectPriority(relativePath) {
  const p0Prefixes = [
    "src-tauri/src/backend/",
    "src-tauri/src/engine/",
    "src-tauri/src/git/",
    "src/features/git-history/",
    "src/features/spec/",
    "src/features/settings/",
  ];

  const p0Explicit = new Set(["src/App.tsx"]);
  if (p0Explicit.has(relativePath) || p0Prefixes.some((prefix) => relativePath.startsWith(prefix))) {
    return "P0";
  }
  if (relativePath.startsWith("src/styles/")) {
    return "P1";
  }
  if (relativePath.startsWith("src/i18n/locales/")) {
    return "P2";
  }
  return "P1";
}

function countLines(content) {
  if (content.length === 0) {
    return 0;
  }
  let newLineCount = 0;
  for (let index = 0; index < content.length; index += 1) {
    if (content[index] === "\n") {
      newLineCount += 1;
    }
  }
  return newLineCount + (content.endsWith("\n") ? 0 : 1);
}

function buildMarkdownReport(results, threshold, generatedAt) {
  const lines = [];
  lines.push("# Large File Baseline");
  lines.push("");
  lines.push(`- Generated at: ${generatedAt}`);
  lines.push(`- Threshold: > ${threshold} lines`);
  lines.push(`- Count: ${results.length}`);
  lines.push("");
  lines.push("| File | Lines | Type | Priority |");
  lines.push("|---|---:|---|---|");
  for (const item of results) {
    lines.push(`| \`${item.path}\` | ${item.lines} | ${item.type} | ${item.priority} |`);
  }
  lines.push("");
  return `${lines.join("\n")}\n`;
}

async function main() {
  const { threshold, mode, markdownOutput, root } = parseArgs(process.argv.slice(2));
  const allFiles = await walkDirectory(root);
  const sourceFiles = allFiles.filter((absolutePath) => TEXT_EXTENSIONS.has(path.extname(absolutePath)));
  const results = [];

  for (const absolutePath of sourceFiles) {
    const content = await fs.readFile(absolutePath, "utf8");
    const lineCount = countLines(content);
    if (lineCount <= threshold) {
      continue;
    }
    const relativePath = path.relative(root, absolutePath).split(path.sep).join("/");
    const extension = path.extname(relativePath);
    results.push({
      path: relativePath,
      lines: lineCount,
      type: detectType(relativePath, extension),
      priority: detectPriority(relativePath),
    });
  }

  results.sort((left, right) => right.lines - left.lines || left.path.localeCompare(right.path));

  console.log(`Large file check: threshold>${threshold}, found=${results.length}`);
  for (const item of results) {
    const message = `${item.path} (${item.lines} lines, ${item.type}, ${item.priority})`;
    if (mode === "warn") {
      console.log(`::warning file=${item.path}::${message}`);
    } else {
      console.log(`- ${message}`);
    }
  }

  if (markdownOutput) {
    const markdownPath = path.resolve(root, markdownOutput);
    await fs.mkdir(path.dirname(markdownPath), { recursive: true });
    const generatedAt = new Date().toISOString();
    const markdown = buildMarkdownReport(results, threshold, generatedAt);
    await fs.writeFile(markdownPath, markdown, "utf8");
    console.log(`Markdown baseline written: ${path.relative(root, markdownPath)}`);
  }

  if (results.length > 0 && mode === "fail") {
    process.exitCode = 1;
  }
}

main().catch((error) => {
  console.error(`large-file-check failed: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
});
