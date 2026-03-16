#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import process from "node:process";

const IS_WINDOWS = process.platform === "win32";

function parseArgs(argv) {
  const config = {
    port: 1420,
    autoKillWorkspaceVite: false,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (token === "--port") {
      const value = Number(argv[index + 1]);
      if (!Number.isInteger(value) || value <= 0 || value > 65535) {
        throw new Error(`Invalid --port value: ${argv[index + 1] ?? "<missing>"}`);
      }
      config.port = value;
      index += 1;
      continue;
    }
    if (token === "--auto-kill-workspace-vite") {
      config.autoKillWorkspaceVite = true;
      continue;
    }

    throw new Error(`Unknown argument: ${token}`);
  }

  return config;
}

function runCommand(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });

  if (result.error) {
    return {
      status: -1,
      stdout: "",
      stderr: String(result.error.message || result.error),
    };
  }

  return {
    status: result.status ?? 0,
    stdout: result.stdout || "",
    stderr: result.stderr || "",
  };
}

function listListeningPids(port) {
  if (IS_WINDOWS) {
    const result = runCommand("netstat", ["-ano", "-p", "tcp"]);
    if (result.status === -1) {
      throw new Error(`Failed to run netstat: ${result.stderr}`);
    }
    if (result.status !== 0) {
      throw new Error(`netstat exited with code ${result.status}: ${result.stderr.trim()}`);
    }
    const pids = new Set();
    for (const rawLine of result.stdout.split("\n")) {
      const line = rawLine.trim();
      if (!line || !line.toUpperCase().startsWith("TCP")) {
        continue;
      }
      const columns = line.split(/\s+/);
      if (columns.length < 5) {
        continue;
      }
      const localAddress = columns[1];
      const state = columns[3];
      const pid = columns[4];
      const colonIndex = localAddress.lastIndexOf(":");
      if (colonIndex < 0) {
        continue;
      }
      const localPort = Number(localAddress.slice(colonIndex + 1));
      if (!Number.isInteger(localPort) || localPort !== port) {
        continue;
      }
      if (state.toUpperCase() !== "LISTENING") {
        continue;
      }
      if (pid) {
        pids.add(pid);
      }
    }
    return Array.from(pids);
  }

  const result = runCommand("lsof", ["-nP", `-iTCP:${port}`, "-sTCP:LISTEN", "-t"]);
  if (result.status === -1) {
    throw new Error(`Failed to run lsof: ${result.stderr}`);
  }
  if (result.status === 1) {
    return [];
  }
  if (result.status !== 0) {
    throw new Error(`lsof exited with code ${result.status}: ${result.stderr.trim()}`);
  }
  return result.stdout.split("\n").map((line) => line.trim()).filter(Boolean);
}

function getProcessCommand(pid) {
  if (IS_WINDOWS) {
    const powershell = runCommand("powershell", [
      "-NoProfile",
      "-Command",
      `(Get-CimInstance Win32_Process -Filter "ProcessId=${String(pid)}").CommandLine`,
    ]);
    if (powershell.status === 0) {
      const command = powershell.stdout.trim();
      if (command) {
        return command;
      }
    }
    const tasklist = runCommand("tasklist", ["/FI", `PID eq ${String(pid)}`, "/FO", "CSV", "/NH"]);
    if (tasklist.status !== 0) {
      return "<unknown>";
    }
    return tasklist.stdout.trim() || "<unknown>";
  }

  const result = runCommand("ps", ["-p", String(pid), "-o", "command="]);
  if (result.status !== 0) {
    return "<unknown>";
  }
  return result.stdout.trim() || "<unknown>";
}

function isWorkspaceViteProcess(command, workspacePath) {
  const normalizedCommand = command.replace(/\\/g, "/").toLowerCase();
  const normalizedWorkspace = workspacePath.replace(/\\/g, "/").toLowerCase();
  return (
    normalizedCommand.includes("vite")
    && normalizedCommand.includes(normalizedWorkspace)
  );
}

function killPid(pid) {
  if (IS_WINDOWS) {
    return runCommand("taskkill", ["/PID", String(pid), "/F"]).status === 0;
  }
  return runCommand("kill", [String(pid)]).status === 0;
}

function ensurePortAvailable(config) {
  const workspacePath = process.cwd();
  const pids = listListeningPids(config.port);

  if (pids.length === 0) {
    console.log(`[dev-port] port ${config.port} is available.`);
    return;
  }

  const processes = pids.map((pid) => ({
    pid,
    command: getProcessCommand(pid),
  }));

  const nonWorkspaceVite = processes.filter(
    (item) => !isWorkspaceViteProcess(item.command, workspacePath),
  );

  if (nonWorkspaceVite.length > 0 || !config.autoKillWorkspaceVite) {
    console.error(`[dev-port] port ${config.port} is already in use:`);
    for (const item of processes) {
      console.error(`  pid=${item.pid} cmd=${item.command}`);
    }
    console.error(
      `[dev-port] resolve it first, then retry. Suggested: kill <pid> for stale dev servers.`,
    );
    process.exit(1);
  }

  for (const item of processes) {
    if (killPid(item.pid)) {
      console.log(`[dev-port] killed stale workspace vite process pid=${item.pid}.`);
    } else {
      console.error(`[dev-port] failed to kill stale process pid=${item.pid}.`);
      process.exit(1);
    }
  }

  const restPids = listListeningPids(config.port);
  if (restPids.length > 0) {
    console.error(`[dev-port] port ${config.port} is still occupied after cleanup: ${restPids.join(", ")}`);
    process.exit(1);
  }

  console.log(`[dev-port] port ${config.port} is available.`);
}

function main() {
  const config = parseArgs(process.argv.slice(2));
  ensurePortAvailable(config);
}

main();
