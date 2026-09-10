import { execFileSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const releaseDir = join(root, "target", "release");
const exePath = join(releaseDir, "EcoPaste.exe");
const assetsPath = join(releaseDir, "assets");
const portableRoot = join(root, "dist", "portable");
const packageDir = join(
  portableRoot,
  `EcoPaste-${pkg.version}-windows-x64-portable`,
);
const zipPath = `${packageDir}.zip`;

if (!existsSync(exePath)) {
  throw new Error(
    "Missing target/release/EcoPaste.exe. Run `pnpm tauri build` first.",
  );
}

rmSync(portableRoot, { force: true, recursive: true });
mkdirSync(packageDir, { recursive: true });

cpSync(exePath, join(packageDir, "EcoPaste.exe"));

if (existsSync(assetsPath)) {
  cpSync(assetsPath, join(packageDir, "assets"), { recursive: true });
}

writeFileSync(join(packageDir, "portable"), "");
writeFileSync(
  join(packageDir, "README.txt"),
  [
    "EcoPaste portable build",
    "",
    "Run EcoPaste.exe directly.",
    "The portable marker file makes EcoPaste store data in ./data next to EcoPaste.exe.",
    "Delete the portable marker to use the normal system app data directory.",
    "",
  ].join("\r\n"),
);

execFileSync(
  "powershell",
  [
    "-NoProfile",
    "-Command",
    `Compress-Archive -Path '${packageDir}\\*' -DestinationPath '${zipPath}' -Force`,
  ],
  { stdio: "inherit" },
);
