import { exists } from "@tauri-apps/plugin-fs";
import {
  writeFiles,
  writeHTML,
  writeImage,
  writeRTF,
  writeText,
} from "tauri-plugin-clipboard-x-api";
import { clipboardStore } from "@/stores/clipboard";
import type { DatabaseSchemaHistory } from "@/types/database";
import { isColor, isEmail, isURL } from "@/utils/is";
import { stripSearchIndex } from "@/utils/pinyin";
import { normalizeTextThreshold } from "@/utils/threshold";
import { paste } from "./paste";
import { hideWindow } from "./window";

export const getClipboardTextSubtype = async (value: string) => {
  try {
    if (isURL(value)) {
      return "url";
    }

    if (isEmail(value)) {
      return "email";
    }

    if (isColor(value)) {
      return "color";
    }

    if (await exists(value)) {
      return "path";
    }
  } catch {
    return;
  }
};

export const writeToClipboard = (data: DatabaseSchemaHistory) => {
  const { type, value, search } = data;
  // 规范化大文本阈值，避免配置越界
  const normalizedTextThreshold = normalizeTextThreshold(
    clipboardStore.content.textThreshold,
  );
  const plainText = stripSearchIndex(search, normalizedTextThreshold);

  switch (type) {
    case "text":
      return writeText(value);
    case "rtf":
      return writeRTF(plainText, value);
    case "html":
      return writeHTML(plainText, value);
    case "image":
      return writeImage(value);
    case "files":
      return writeFiles(value);
  }
};

export const pasteToClipboard = async (
  data: DatabaseSchemaHistory,
  asPlain?: boolean,
) => {
  const { type, value, search } = data;
  const { pastePlain } = clipboardStore.content;
  // 规范化大文本阈值，避免配置越界
  const normalizedTextThreshold = normalizeTextThreshold(
    clipboardStore.content.textThreshold,
  );

  if (asPlain ?? pastePlain) {
    if (type === "files") {
      await writeText(value.join("\n"));
    } else {
      await writeText(stripSearchIndex(search, normalizedTextThreshold));
    }
  } else {
    await writeToClipboard(data);
  }

  await paste();

  // 粘贴完成后隐藏窗口
  hideWindow();
};
