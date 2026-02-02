import { downloadDir } from "@tauri-apps/api/path";
import { copyFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import type { MenuProps } from "antd";
import { find, isArray, remove } from "es-toolkit/compat";
import { useContext } from "react";
import { useTranslation } from "react-i18next";
import { useSnapshot } from "valtio";
import { deleteHistory, updateHistory } from "@/database/history";
import { MainContext } from "@/pages/Main";
import type { ItemProps } from "@/pages/Main/components/HistoryList/components/Item";
import { pasteToClipboard, writeToClipboard } from "@/plugins/clipboard";
import { clipboardStore } from "@/stores/clipboard";
import { globalStore } from "@/stores/global";
import { isMac } from "@/utils/is";
import { join } from "@/utils/path";

// 标志：是否有删除确认框正在显示
let isDeleteModalVisible = false;

interface UseContextMenuProps extends ItemProps {
  handleNext: () => void;
}

export const useContextMenu = (props: UseContextMenuProps) => {
  const { data, deleteModal, handleNote, handleNext } = props;
  const { id, type, value, group, favorite, subtype } = data;
  const { t } = useTranslation();
  const { env } = useSnapshot(globalStore);
  const { rootState } = useContext(MainContext);

  const pasteAsText = () => {
    return pasteToClipboard(data, true);
  };

  const handleFavorite = async () => {
    const nextFavorite = !favorite;

    const matched = find(rootState.list, { id });

    if (!matched) return;

    matched.favorite = nextFavorite;

    updateHistory(id, { favorite: nextFavorite });
  };

  const openToBrowser = () => {
    if (type !== "text") return;

    const url = value.startsWith("http") ? value : `http://${value}`;

    openUrl(url);
  };

  const exportToFile = async () => {
    if (isArray(value)) return;

    const extname = type === "text" ? "txt" : type;
    const fileName = `${env.appName}_${id}.${extname}`;
    const path = join(await downloadDir(), fileName);

    await writeTextFile(path, value);

    revealItemInDir(path);
  };

  const downloadImage = async () => {
    if (type !== "image") return;

    const fileName = `${env.appName}_${id}.png`;
    const path = join(await downloadDir(), fileName);

    await copyFile(value, path);

    revealItemInDir(path);
  };

  const openToFinder = () => {
    if (type === "text") {
      return revealItemInDir(value);
    }

    const [file] = value;

    revealItemInDir(file);
  };

  const handleDelete = async () => {
    const matched = find(rootState.list, { id });

    if (!matched) return;

    let confirmed = true;

    if (clipboardStore.content.deleteConfirm) {
      // 如果已有确认框正在显示，直接返回
      if (isDeleteModalVisible) return;

      isDeleteModalVisible = true;

      confirmed = await deleteModal.confirm({
        afterClose() {
          isDeleteModalVisible = false;
          // 关闭确认框后焦点还在，需要手动取消焦点
          (document.activeElement as HTMLElement)?.blur();
        },
        centered: true,
        content: t("clipboard.hints.delete_modal_content"),
      });
    }

    if (!confirmed) return;

    if (id === rootState.activeId) {
      handleNext();
    }

    remove(rootState.list, { id });

    deleteHistory(data);
  };

  // 构建 Antd Dropdown 菜单项
  const getMenuItems = (): MenuProps["items"] => {
    const allItems = [
      {
        key: "copy",
        label: t("clipboard.button.context_menu.copy"),
        onClick: () => writeToClipboard(data),
      },
      {
        key: "note",
        label: t("clipboard.button.context_menu.note"),
        onClick: handleNote,
      },
      {
        hide: type !== "html" && type !== "rtf",
        key: "paste_as_text",
        label: t("clipboard.button.context_menu.paste_as_plain_text"),
        onClick: pasteAsText,
      },
      {
        hide: type !== "files",
        key: "paste_as_path",
        label: t("clipboard.button.context_menu.paste_as_path"),
        onClick: pasteAsText,
      },
      {
        key: "favorite",
        label: favorite
          ? t("clipboard.button.context_menu.unfavorite")
          : t("clipboard.button.context_menu.favorite"),
        onClick: handleFavorite,
      },
      {
        hide: subtype !== "url",
        key: "open_browser",
        label: t("clipboard.button.context_menu.open_in_browser"),
        onClick: openToBrowser,
      },
      {
        hide: subtype !== "email",
        key: "send_email",
        label: t("clipboard.button.context_menu.send_email"),
        onClick: () => openUrl(`mailto:${value}`),
      },
      {
        hide: group !== "text",
        key: "export_file",
        label: t("clipboard.button.context_menu.export_as_file"),
        onClick: exportToFile,
      },
      {
        hide: type !== "image",
        key: "download_image",
        label: t("clipboard.button.context_menu.download_image"),
        onClick: downloadImage,
      },
      {
        hide: type !== "files" && subtype !== "path",
        key: "show_in_folder",
        label: isMac
          ? t("clipboard.button.context_menu.show_in_finder")
          : t("clipboard.button.context_menu.show_in_file_explorer"),
        onClick: openToFinder,
      },
      {
        danger: true,
        key: "delete",
        label: t("clipboard.button.context_menu.delete"),
        onClick: handleDelete,
      },
    ];

    return allItems
      .filter((item) => !("hide" in item && item.hide))
      .map(({ hide, ...item }) => item);
  };

  return {
    getMenuItems,
    handleDelete,
    handleFavorite,
  };
};
