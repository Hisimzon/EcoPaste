import { getName } from "@tauri-apps/api/app";
import { appDataDir, resourceDir, sep } from "@tauri-apps/api/path";
import { exists } from "@tauri-apps/plugin-fs";
import { last } from "es-toolkit";
import { globalStore } from "@/stores/global";
import { isDev } from "./is";

/**
 * 拼接文件路径
 * @param paths 路径数组
 */
export function join(...paths: string[]) {
  const joinPaths = paths.map((path, index) => {
    if (index === 0) {
      return path.replace(new RegExp(`${sep()}+$`), "");
    }

    return path.replace(new RegExp(`^${sep()}+|${sep()}+$`, "g"), "");
  });

  return joinPaths.join(sep());
}

/**
 * 获取存储数据的目录
 */
export const getSaveDataPath = () => {
  return join(globalStore.env.saveDataDir!);
};

/**
 * 是否启用免安装便携模式
 */
export const isPortableMode = async () => {
  return exists(join(await resourceDir(), "portable"));
};

/**
 * 获取默认存储数据的目录
 */
export const getDefaultSaveDataPath = async () => {
  if (await isPortableMode()) {
    return join(await resourceDir(), "data");
  }

  return appDataDir();
};

/**
 * 获取数据库文件存储路径
 */
export const getSaveDatabasePath = async () => {
  const appName = await getName();
  const extname = isDev() ? "dev.db" : "db";

  return join(getSaveDataPath(), `${appName}.${extname}`);
};

/**
 * 获取存储图片的路径
 */
export const getSaveImagePath = () => {
  return join(getSaveDataPath(), "images");
};

/**
 * 存储数据的目录名
 */
export const getSaveDataDirName = () => {
  return last(getSaveDataPath().split(sep())) as string;
};

/**
 * 存储配置项的路径
 * @param backup 是否是备份数据
 */
export const getSaveStorePath = async (backup = false) => {
  const extname = isDev() ? "dev.json" : "json";

  if (backup) {
    return join(getSaveDataPath(), `.store-backup.${extname}`);
  }

  return join(await getDefaultSaveDataPath(), `.store.${extname}`);
};

/**
 * 存储窗口位置的路径
 */
export const getSaveWindowStatePath = async () => {
  const extname = isDev() ? "dev.json" : "json";

  return join(await getDefaultSaveDataPath(), `.window-state.${extname}`);
};
