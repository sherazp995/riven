import * as path from "path";
import { workspace, ExtensionContext, window } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(context: ExtensionContext): Promise<void> {
  const config = workspace.getConfiguration("riven");
  const configured = config.get<string>("server.path")?.trim();

  const command = configured && configured.length > 0
    ? configured
    : defaultServerPath(context);

  const serverOptions: ServerOptions = {
    run: { command, transport: TransportKind.stdio },
    debug: { command, transport: TransportKind.stdio },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "riven" },
      { scheme: "untitled", language: "riven" },
    ],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/*.{riven,rvn}"),
    },
  };

  client = new LanguageClient("riven", "Riven LSP", serverOptions, clientOptions);

  try {
    await client.start();
  } catch (err) {
    window.showErrorMessage(
      `Failed to start riven-lsp (${command}). Set 'riven.server.path' in settings. ${err}`,
    );
  }
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

function defaultServerPath(_context: ExtensionContext): string {
  const ext = process.platform === "win32" ? ".exe" : "";
  const ws = workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (ws) {
    return path.join(ws, "target", "release", `riven-lsp${ext}`);
  }
  return `riven-lsp${ext}`;
}
